"""
HTTP-only tee time bot for Blue Hill CC (Northstar/Liferay JSF portal).
Drives JSF AJAX calls directly without a browser.
"""
import logging
import os
import re
from datetime import datetime, timedelta
from urllib.parse import urljoin
from xml.etree import ElementTree as ET

import httpx
from bs4 import BeautifulSoup

logger = logging.getLogger(__name__)

_PORTLET = "teeTimePortlet_WAR_northstarportlet"
_NS = f"_{_PORTLET}_"
_FORM = f"{_NS}:teeTimeForm"
_DEFAULT_URL = "https://www.bluehillcc.com/group/pages/teetime"


class TeeTimeBot:
    def __init__(self, cfg: dict, username: str, password: str, dry_run: bool = False):
        self.cfg = cfg
        self.dry_run = dry_run
        self._username = username
        self._password = password
        self._url = cfg.get("club", {}).get("teetime_url", _DEFAULT_URL)

    def run(self) -> str | None:
        with httpx.Client(
            follow_redirects=True,
            timeout=30,
            headers={
                "User-Agent": (
                    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
                    "AppleWebKit/537.36 (KHTML, like Gecko) "
                    "Chrome/124.0.0.0 Safari/537.36"
                ),
            },
        ) as client:
            self._login(client)
            return self._book(client)

    # ── Auth ────────────────────────────────────────────────────────────────────

    def _login(self, client: httpx.Client) -> None:
        r = client.get(self._url)
        if _is_login_page(r):
            logger.info("Redirected to login — submitting credentials")
            r = self._submit_login_form(client, r)
            if _is_login_page(r):
                raise RuntimeError("Login failed — verify credentials for this member")
            logger.info("Authenticated")
        else:
            logger.info("Session already valid")

    def _submit_login_form(self, client: httpx.Client, page: httpx.Response) -> httpx.Response:
        soup = BeautifulSoup(page.text, "html.parser")
        form = soup.find("form")
        if not form:
            raise RuntimeError("Login form not found on page")

        action = form.get("action") or str(page.url)
        if not action.startswith("http"):
            action = urljoin(str(page.url), action)

        payload = {
            inp["name"]: inp.get("value", "")
            for inp in form.find_all("input", type="hidden")
            if inp.get("name")
        }
        for inp in form.find_all("input"):
            name = inp.get("name", "")
            itype = inp.get("type", "text").lower()
            if itype in ("email", "text") and re.search(r"(login|email|user)", name, re.I):
                payload[name] = self._username
            elif itype == "password":
                payload[name] = self._password

        return client.post(action, data=payload)

    # ── Booking ─────────────────────────────────────────────────────────────────

    def _book(self, client: httpx.Client) -> str | None:
        prefs = self.cfg["preferences"]
        target = datetime.now() + timedelta(days=prefs["days_in_advance"])
        logger.info("Targeting %s", target.strftime("%A, %B %-d"))

        r = client.get(self._url)
        p_auth = _extract_p_auth(r.text)
        viewstate = _extract_viewstate(r.text)

        viewstate, slots_html = self._select_date(client, p_auth, viewstate, target, r.text)

        slot_id, label = _find_slot(slots_html, prefs)
        if not slot_id:
            logger.info(
                "No slots available between %s–%s",
                prefs["earliest_time"],
                prefs["latest_time"],
            )
            return None

        logger.info("Slot found: %s", label)

        if self.dry_run:
            logger.info("Dry run — skipping reservation")
        else:
            self._reserve(client, p_auth, viewstate, slot_id, prefs.get("players", 4))

        return f"{target.strftime('%A, %b %-d')} at {label}"

    # ── AJAX helpers ─────────────────────────────────────────────────────────────

    def _ajax(
        self, client: httpx.Client, p_auth: str, viewstate: str, body: dict
    ) -> httpx.Response:
        params = {
            "p_auth": p_auth,
            "p_p_id": _PORTLET,
            "p_p_lifecycle": "1",
            "p_p_state": "normal",
            "p_p_mode": "view",
        }
        data = {_FORM: _FORM, "javax.faces.ViewState": viewstate, **body}
        r = client.post(
            self._url,
            params=params,
            data=data,
            headers={
                "Faces-Request": "partial/ajax",
                "X-Requested-With": "XMLHttpRequest",
            },
        )
        r.raise_for_status()
        return r

    def _select_date(
        self,
        client: httpx.Client,
        p_auth: str,
        viewstate: str,
        target: datetime,
        page_html: str,
    ) -> tuple[str, str]:
        tab_id = _find_date_tab_id(page_html, target)
        if not tab_id:
            logger.warning(
                "Date tab for %s not found — using current page slots",
                target.strftime("%b %-d"),
            )
            return viewstate, page_html

        logger.info("Selecting date tab: %s", tab_id)
        r = self._ajax(client, p_auth, viewstate, {
            "javax.faces.partial.ajax": "true",
            "javax.faces.source": tab_id,
            "javax.faces.partial.execute": "@all",
            "javax.faces.partial.render": f"{_FORM}:teeTimeCourses",
            tab_id: tab_id,
        })
        new_vs = _jsf_viewstate(r.text) or viewstate
        return new_vs, r.text

    def _reserve(
        self,
        client: httpx.Client,
        p_auth: str,
        viewstate: str,
        button_id: str,
        players: int,
    ) -> None:
        logger.info("Clicking Reserve: %s", button_id)
        r = self._ajax(client, p_auth, viewstate, {
            "javax.faces.partial.ajax": "true",
            "javax.faces.source": button_id,
            "javax.faces.partial.execute": "@all",
            "javax.faces.partial.render": _FORM,
            button_id: button_id,
        })
        new_vs = _jsf_viewstate(r.text) or viewstate
        content = _unwrap_jsf_partial(r.text)

        # If the server already confirms, we're done
        if re.search(r"(confirm.*complet|success|thank|booked)", content, re.I):
            logger.info("Booking confirmed")
            return

        # Otherwise look for a second-step confirm button in the dialog HTML
        soup = BeautifulSoup(content, "html.parser")
        confirm_btn = _find_confirm_button(soup)
        if not confirm_btn or not confirm_btn.get("id"):
            logger.warning("No second-step confirm button found — verify booking manually")
            return

        confirm_id = confirm_btn["id"]
        logger.info("Clicking confirm button: %s", confirm_id)

        extra: dict = {
            "javax.faces.partial.ajax": "true",
            "javax.faces.source": confirm_id,
            "javax.faces.partial.execute": "@all",
            "javax.faces.partial.render": _FORM,
            confirm_id: confirm_id,
        }
        # Fill players dropdown if present in the dialog
        players_sel = soup.find("select", id=re.compile(r"(player|numPlayer|guest)", re.I))
        if players_sel and players_sel.get("id"):
            extra[players_sel["id"]] = str(players)

        self._ajax(client, p_auth, new_vs, extra)
        logger.info("Booking submitted")


# ── Page parsing helpers ───────────────────────────────────────────────────────

def _is_login_page(r: httpx.Response) -> bool:
    return "/login" in str(r.url) or "LoginPortlet" in r.text


def _extract_p_auth(html: str) -> str:
    m = re.search(r"Liferay\.authToken\s*=\s*['\"]([^'\"]+)['\"]", html)
    if not m:
        raise RuntimeError("Liferay.authToken not found — not authenticated?")
    return m.group(1)


def _extract_viewstate(html: str) -> str:
    soup = BeautifulSoup(html, "html.parser")
    el = soup.find("input", {"name": "javax.faces.ViewState"})
    if not el:
        raise RuntimeError("javax.faces.ViewState not found in page")
    return el["value"]


def _jsf_viewstate(xml_text: str) -> str | None:
    """Extract the refreshed ViewState from a JSF partial-response envelope."""
    try:
        root = ET.fromstring(xml_text.strip())
        for el in root.iter("update"):
            if el.get("id") == "javax.faces.ViewState":
                return el.text
    except ET.ParseError:
        pass
    return None


def _unwrap_jsf_partial(text: str) -> str:
    """Extract inner HTML from a JSF <partial-response> XML body."""
    if "<partial-response" not in text and not text.lstrip().startswith("<?xml"):
        return text
    try:
        root = ET.fromstring(text.strip())
        parts = [
            el.text or ""
            for el in root.iter("update")
            if "ViewState" not in el.get("id", "")
        ]
        return " ".join(parts)
    except ET.ParseError:
        return text


def _find_date_tab_id(html: str, target: datetime) -> str | None:
    """
    Find the JSF component ID of the date tab matching target.
    Northstar renders a 7-day horizontal strip; each day is a clickable cell.
    """
    soup = BeautifulSoup(html, "html.parser")

    needles = [
        target.strftime("%-m/%-d"),   # "6/9"
        target.strftime("%m/%d"),     # "06/09"
        target.strftime("%b %-d"),    # "Jun 9"
        target.strftime("%B %-d"),    # "June 9"
    ]
    iso = target.strftime("%Y-%m-%d")

    for tag in soup.find_all(["a", "button", "td", "th", "li", "span", "div"]):
        text = tag.get_text(" ", strip=True)
        if any(n in text for n in needles) or tag.get("data-date") == iso:
            node = tag
            for _ in range(5):
                if node is None:
                    break
                cid = node.get("id", "")
                if cid and ":" in cid:  # JSF component ids use ":" as separator
                    return cid
                node = getattr(node, "parent", None)

    return None


def _find_slot(html: str, prefs: dict) -> tuple[str | None, str | None]:
    """
    Return (component_id, label) for the first Reserve button whose time
    falls within [earliest_time, latest_time].
    """
    earliest = datetime.strptime(prefs["earliest_time"], "%H:%M").time()
    latest = datetime.strptime(prefs["latest_time"], "%H:%M").time()

    content = _unwrap_jsf_partial(html)
    soup = BeautifulSoup(content, "html.parser")

    time_re = re.compile(r"\b\d{1,2}:\d{2}\s*[AaPp][Mm]\b")

    for btn in soup.find_all(attrs={"id": re.compile(r"reserve_button")}):
        container = btn.find_parent(["tr", "li", "div"]) or btn.parent
        m = time_re.search(container.get_text(" ") if container else "")
        if not m:
            continue
        t = _parse_time(m.group().strip())
        if t and earliest <= t <= latest:
            return btn["id"], m.group().strip()

    return None, None


def _find_confirm_button(soup: BeautifulSoup):
    """Locate a confirmation/submit button in a booking dialog."""
    for btn in soup.find_all(["button", "input"]):
        bid = btn.get("id", "").lower()
        btext = btn.get_text(strip=True).lower()
        if any(k in bid or k in btext for k in ("confirm", "book", "submit")):
            if "cancel" not in bid and "cancel" not in btext:
                return btn
    return None


def _parse_time(s: str):
    for fmt in ("%I:%M %p", "%I:%M%p", "%-I:%M %p"):
        try:
            return datetime.strptime(s.upper(), fmt).time()
        except ValueError:
            continue
    return None
