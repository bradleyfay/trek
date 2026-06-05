"""
Entry point. Run normally to start the scheduler; use --run-now to fire
immediately (handy for testing your selector config before deploying).
"""

import argparse
import logging
import os
import sys

import yaml
from dotenv import load_dotenv
from apscheduler.schedulers.blocking import BlockingScheduler
from apscheduler.triggers.cron import CronTrigger
from twilio.rest import Client

from bot import TeeTimeBot

load_dotenv()  # loads .env when running locally; Railway uses env vars directly

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(message)s",
    stream=sys.stdout,
)
logger = logging.getLogger(__name__)


def load_config() -> dict:
    path = os.getenv("CONFIG_PATH", "config.yaml")
    with open(path) as f:
        return yaml.safe_load(f)


def send_sms(message: str) -> None:
    client = Client(os.environ["TWILIO_ACCOUNT_SID"], os.environ["TWILIO_AUTH_TOKEN"])
    client.messages.create(
        body=message,
        from_=os.environ["TWILIO_FROM_NUMBER"],
        to=os.environ["TWILIO_TO_NUMBER"],
    )
    logger.info("SMS sent: %s", message)


def _member_prefs(cfg: dict, member: dict) -> dict:
    """Merge global preferences with any per-member overrides."""
    prefs = dict(cfg["preferences"])
    for key in ("days_in_advance", "earliest_time", "latest_time", "players"):
        if key in member:
            prefs[key] = member[key]
    return prefs


def run_member(cfg: dict, member: dict, dry_run: bool = False) -> None:
    name = member.get("name", member["env_prefix"])
    prefix = member["env_prefix"]

    username = os.environ[f"{prefix}_USERNAME"]
    password = os.environ[f"{prefix}_PASSWORD"]

    merged_cfg = {**cfg, "preferences": _member_prefs(cfg, member)}
    bot = TeeTimeBot(merged_cfg, username, password, dry_run=dry_run)

    try:
        result = bot.run()
        tag = " [DRY RUN]" if dry_run else ""
        if result:
            send_sms(f"{name} — Tee time booked{tag}: {result} ⛳")
        else:
            send_sms(f"{name} — No tee times found in preferred window. Book manually!")
    except Exception as exc:
        logger.exception("Bot run failed for %s", name)
        send_sms(f"{name} — Tee time bot error: {exc}")


def run_all(cfg: dict, dry_run: bool = False) -> None:
    members = cfg.get("members", [])
    if not members:
        logger.error("No members defined in config.yaml")
        return
    for member in members:
        run_member(cfg, member, dry_run=dry_run)


def main(cfg: dict) -> None:
    schedule = cfg["schedule"]
    hour, minute = schedule["run_time"].split(":")
    tz = schedule["timezone"]
    days_str = ",".join(str(d) for d in schedule["run_on_days"])

    scheduler = BlockingScheduler(timezone=tz)
    scheduler.add_job(
        run_all,
        CronTrigger(day_of_week=days_str, hour=int(hour), minute=int(minute), timezone=tz),
        args=[cfg],
        name="book_tee_times",
    )
    logger.info(
        "Scheduler running. Will fire at %s %s on day(s): %s",
        schedule["run_time"],
        tz,
        schedule["run_on_days"],
    )
    scheduler.start()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Tee time booking bot")
    parser.add_argument(
        "--run-now",
        action="store_true",
        help="Run the bot immediately instead of waiting for the schedule",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Find a slot but stop before clicking Confirm (implies --run-now)",
    )
    args = parser.parse_args()

    cfg = load_config()

    if args.run_now or args.dry_run:
        run_all(cfg, dry_run=args.dry_run)
    else:
        main(cfg)
