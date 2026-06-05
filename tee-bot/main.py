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


def run_bot(cfg: dict, dry_run: bool = False) -> None:
    bot = TeeTimeBot(cfg, dry_run=dry_run)
    try:
        result = bot.run()
        if result:
            tag = " [DRY RUN]" if dry_run else ""
            send_sms(f"Tee time booked{tag}: {result} ⛳")
        else:
            send_sms("No tee times found in your preferred window. Book manually!")
    except Exception as exc:
        logger.exception("Bot run failed")
        send_sms(f"Tee time bot error: {exc}")


def main(cfg: dict) -> None:
    schedule = cfg["schedule"]
    hour, minute = schedule["run_time"].split(":")
    tz = schedule["timezone"]

    # run_on_days uses 0=Monday … 6=Sunday (Python convention)
    days_str = ",".join(str(d) for d in schedule["run_on_days"])

    scheduler = BlockingScheduler(timezone=tz)
    scheduler.add_job(
        run_bot,
        CronTrigger(day_of_week=days_str, hour=int(hour), minute=int(minute), timezone=tz),
        args=[cfg],
        name="book_tee_time",
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
        run_bot(cfg, dry_run=args.dry_run)
    else:
        main(cfg)
