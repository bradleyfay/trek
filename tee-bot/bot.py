import asyncio
import logging
import os
from datetime import datetime, timedelta

from playwright.async_api import Page, TimeoutError as PlaywrightTimeout, async_playwright

logger = logging.getLogger(__name__)

DAY_NAMES = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]


class TeeTimeBot:
    def __init__(self, cfg: dict, dry_run: bool = False):
        self.cfg = cfg
        self.dry_run = dry_run
        self.headless = os.getenv("HEADLESS", "true").lower() != "false"

    def run(self) -> str | None:
        return asyncio.run(self._run())

    async def _run(self) -> str | None:
        async with async_playwright() as p:
            browser = await p.chromium.launch(headless=self.headless)
            context = await browser.new_context()
            page = await context.new_page()
            try:
                await self._login(page)
                return await self._book(page)
            finally:
                await browser.close()

    async def _login(self, page: Page) -> None:
        login = self.cfg["login"]
        logger.info("Navigating to login page")
        await page.goto(login["url"])
        await page.fill(login["username_selector"], os.environ["CLUB_USERNAME"])
        await page.fill(login["password_selector"], os.environ["CLUB_PASSWORD"])
        await page.click(login["submit_selector"])
        await page.wait_for_selector(login["success_selector"], timeout=15_000)
        logger.info("Login succeeded")

    async def _book(self, page: Page) -> str | None:
        booking = self.cfg["booking"]
        prefs = self.cfg["preferences"]

        target_date = datetime.now() + timedelta(days=prefs["days_in_advance"])
        target_date_str = target_date.strftime(booking.get("date_format", "%Y-%m-%d"))
        logger.info("Targeting tee time on %s", target_date_str)

        await page.goto(booking["url"])

        # Select the date — supports either a text input or a clickable calendar cell.
        if booking.get("date_input_selector"):
            await page.fill(booking["date_input_selector"], target_date_str)
        elif booking.get("date_click_selector"):
            selector = booking["date_click_selector"].format(date=target_date_str)
            await page.click(selector)

        await page.wait_for_selector(booking["time_slot_selector"], timeout=10_000)

        earliest = datetime.strptime(prefs["earliest_time"], "%H:%M").time()
        latest = datetime.strptime(prefs["latest_time"], "%H:%M").time()
        time_fmt = booking.get("time_display_format", "%I:%M %p")

        slots = await page.query_selector_all(booking["time_slot_selector"])
        chosen = None
        chosen_label = None

        for slot in slots:
            raw = (await slot.text_content() or "").strip()
            # Try configured format first, then a few common fallbacks.
            for fmt in [time_fmt, "%I:%M %p", "%H:%M", "%I:%M%p"]:
                try:
                    slot_time = datetime.strptime(raw, fmt).time()
                    if earliest <= slot_time <= latest:
                        chosen = slot
                        chosen_label = raw
                    break
                except ValueError:
                    continue
            if chosen:
                break

        if not chosen:
            logger.warning("No slots found between %s and %s", prefs["earliest_time"], prefs["latest_time"])
            return None

        await chosen.click()
        logger.info("Selected slot: %s", chosen_label)

        if booking.get("players_selector") and prefs.get("players"):
            await page.select_option(booking["players_selector"], str(prefs["players"]))

        if self.dry_run:
            logger.info("Dry run — skipping confirm click")
        else:
            await page.click(booking["confirm_selector"])
            if booking.get("confirmation_selector"):
                await page.wait_for_selector(booking["confirmation_selector"], timeout=15_000)

        result = f"{target_date.strftime('%A, %b %-d')} at {chosen_label}"
        logger.info("Booked: %s%s", result, " (DRY RUN)" if self.dry_run else "")
        return result
