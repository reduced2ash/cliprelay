from __future__ import annotations

import asyncio
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

import httpx
from telethon import TelegramClient
from telethon.errors import SessionPasswordNeededError
from telethon.sessions import StringSession

from .secrets import SecretStore


class TelegramError(RuntimeError):
    pass


class TelegramPasswordRequired(TelegramError):
    pass


@dataclass(slots=True)
class TelegramDelivery:
    message_id: str
    link: str
    detail: str


class TelegramBotService:
    def __init__(self, secrets: SecretStore):
        self.secrets = secrets

    @property
    def token(self) -> str:
        return self.secrets.get("telegram_bot_token")

    async def _token(self) -> str:
        return await asyncio.to_thread(
            self.secrets.get,
            "telegram_bot_token",
        )

    async def validate(self, token: str) -> dict[str, Any]:
        if not token or ":" not in token:
            raise TelegramError("Enter the complete bot token from @BotFather.")
        async with httpx.AsyncClient(timeout=30) as client:
            response = await client.get(f"https://api.telegram.org/bot{token}/getMe")
        payload = response.json()
        if not response.is_success or not payload.get("ok"):
            raise TelegramError(payload.get("description") or "Telegram could not validate this bot token.")
        await asyncio.to_thread(
            self.secrets.set,
            "telegram_bot_token",
            token,
        )
        return payload["result"]

    async def validate_destination(self, destination: str) -> dict[str, Any]:
        token = await self._token()
        if not token:
            raise TelegramError("Add a Telegram bot token first.")
        if not destination:
            raise TelegramError("Enter a channel username or numeric chat ID.")
        async with httpx.AsyncClient(timeout=30) as client:
            response = await client.post(
                f"https://api.telegram.org/bot{token}/getChat", data={"chat_id": destination}
            )
        payload = response.json()
        if not response.is_success or not payload.get("ok"):
            raise TelegramError(payload.get("description") or "Telegram could not find that destination.")
        return payload["result"]

    async def send_video(
        self,
        path: str | Path,
        caption: str,
        destination: str,
        progress: Callable[[float, str], None] | None = None,
    ) -> TelegramDelivery:
        target = Path(path)
        token = await self._token()
        if not token:
            raise TelegramError("The Telegram bot is not configured.")
        if not target.is_file():
            raise TelegramError("The prepared video is no longer available.")
        if target.stat().st_size > 50 * 1024 * 1024:
            raise TelegramError("This file exceeds the standard Telegram bot upload limit. Compress it or use Personal account.")
        if progress:
            progress(0.05, "Uploading to Telegram")
        async with httpx.AsyncClient(timeout=None) as client:
            with target.open("rb") as stream:
                response = await client.post(
                    f"https://api.telegram.org/bot{token}/sendVideo",
                    data={
                        "chat_id": destination,
                        "caption": caption[:1024],
                        "supports_streaming": "true",
                    },
                    files={"video": (target.name, stream, "video/mp4")},
                )
        payload = response.json()
        if not response.is_success or not payload.get("ok"):
            raise TelegramError(payload.get("description") or "Telegram could not send this video.")
        message = payload["result"]
        chat = message.get("chat") or {}
        username = chat.get("username")
        message_id = str(message.get("message_id", ""))
        link = f"https://t.me/{username}/{message_id}" if username and message_id else ""
        if progress:
            progress(1.0, "Telegram sent")
        return TelegramDelivery(message_id, link, "Sent through bot")


class TelegramPersonalService:
    def __init__(self, secrets: SecretStore):
        self.secrets = secrets
        self._login_client: TelegramClient | None = None
        self._phone = ""
        self._api_hash = ""
        self._api_id = 0
        self._awaiting_password = False

    def _session(self) -> str:
        return self.secrets.get("telegram_personal_session")

    async def begin_login(self, api_id: int, api_hash: str, phone: str) -> None:
        if api_id <= 0 or not api_hash.strip() or not phone.strip():
            raise TelegramError("Enter your Telegram API ID, API hash, and phone number.")
        if self._login_client:
            await self._login_client.disconnect()
        self._api_id, self._api_hash, self._phone = api_id, api_hash, phone
        self._awaiting_password = False
        self._login_client = TelegramClient(StringSession(), api_id, api_hash)
        await self._login_client.connect()
        await self._login_client.send_code_request(phone)

    async def complete_login(self, code: str = "", password: str = "") -> str:
        client = self._login_client
        if not client:
            raise TelegramError("Request a Telegram sign-in code first.")
        try:
            if self._awaiting_password:
                if not password:
                    raise TelegramPasswordRequired("Enter this account's two-step verification password.")
                await client.sign_in(password=password)
            else:
                await client.sign_in(phone=self._phone, code=code)
        except SessionPasswordNeededError as exc:
            self._awaiting_password = True
            if password:
                await client.sign_in(password=password)
            else:
                raise TelegramPasswordRequired("This Telegram account requires its two-step verification password.") from exc
        if not await client.is_user_authorized():
            raise TelegramError("Telegram sign-in did not complete.")
        session = client.session.save()
        await asyncio.to_thread(
            self.secrets.set,
            "telegram_personal_session",
            str(session),
        )
        await asyncio.to_thread(
            self.secrets.set,
            "telegram_api_hash",
            self._api_hash,
        )
        user = await client.get_me()
        display = " ".join(part for part in [getattr(user, "first_name", ""), getattr(user, "last_name", "")] if part)
        await client.disconnect()
        self._login_client = None
        self._awaiting_password = False
        return display or getattr(user, "username", "Telegram account")

    async def _client(self, api_id: int) -> TelegramClient:
        session = await asyncio.to_thread(
            self.secrets.get,
            "telegram_personal_session",
        )
        api_hash = await asyncio.to_thread(
            self.secrets.get,
            "telegram_api_hash",
        )
        if not session or not api_hash or not api_id:
            raise TelegramError("The personal Telegram account is not signed in.")
        client = TelegramClient(StringSession(session), api_id, api_hash)
        await client.connect()
        if not await client.is_user_authorized():
            await client.disconnect()
            raise TelegramError("The Telegram session expired. Sign in again in Settings.")
        return client

    async def dialogs(self, api_id: int) -> list[dict[str, Any]]:
        client = await self._client(api_id)
        try:
            dialogs = await client.get_dialogs(limit=250)
            return [
                {
                    "id": str(dialog.id),
                    "name": dialog.name or str(dialog.id),
                    "title": dialog.name or str(dialog.id),
                    "isChannel": bool(dialog.is_channel),
                    "isGroup": bool(dialog.is_group),
                }
                for dialog in dialogs
                if not dialog.is_user or not getattr(dialog.entity, "bot", False)
            ]
        finally:
            await client.disconnect()

    async def send_video(
        self,
        api_id: int,
        destination: str,
        path: str | Path,
        caption: str,
        progress: Callable[[float, str], None] | None = None,
    ) -> TelegramDelivery:
        client = await self._client(api_id)
        target: str | int = int(destination) if destination.lstrip("-").isdigit() else destination

        def callback(current: int, total: int) -> None:
            if progress and total:
                progress(current / total, "Uploading to Telegram")

        try:
            message = await client.send_file(
                target, str(path), caption=caption[:4096], supports_streaming=True,
                progress_callback=callback,
            )
            entity = await client.get_entity(target)
            username = getattr(entity, "username", None)
            message_id = str(getattr(message, "id", ""))
            link = f"https://t.me/{username}/{message_id}" if username and message_id else ""
            if progress:
                progress(1.0, "Telegram sent")
            return TelegramDelivery(message_id, link, "Sent through personal account")
        finally:
            await client.disconnect()

    async def sign_out(self, api_id: int) -> None:
        session = await asyncio.to_thread(
            self.secrets.get,
            "telegram_personal_session",
        )
        api_hash = await asyncio.to_thread(
            self.secrets.get,
            "telegram_api_hash",
        )
        if session and api_hash and api_id:
            try:
                client = TelegramClient(StringSession(session), api_id, api_hash)
                await client.connect()
                await client.log_out()
            except Exception:
                pass
        await asyncio.to_thread(
            self.secrets.delete,
            "telegram_personal_session",
        )
        await asyncio.to_thread(
            self.secrets.delete,
            "telegram_api_hash",
        )
