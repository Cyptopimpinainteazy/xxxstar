"""
GPU Swarm Social Agents
Integration with Twitter/X, Telegram, Discord for agent actions
"""

import os
import json
import logging
import asyncio
from typing import Dict, Any, Optional, List, Callable
from dataclasses import dataclass, field
from enum import Enum
from datetime import datetime
from abc import ABC, abstractmethod

logger = logging.getLogger(__name__)


class ActionStatus(Enum):
    """Status of social media action"""
    PENDING = "pending"
    EXECUTING = "executing"
    SUCCESS = "success"
    FAILED = "failed"
    RETRYING = "retrying"


@dataclass
class SocialAction:
    """Represents a social media action"""
    action_id: str
    platform: str
    action_type: str  # "post", "comment", "react", "direct_message"
    target: str  # Tweet ID, channel, user ID
    content: str
    metadata: Dict[str, Any] = field(default_factory=dict)
    status: ActionStatus = ActionStatus.PENDING
    timestamp: float = field(default_factory=__import__('time').time)
    retries: int = 0
    max_retries: int = 3


class SocialPlatformAdapter(ABC):
    """Abstract base for social platform adapters"""

    @abstractmethod
    async def authenticate(self):
        """Authenticate with platform"""
        pass

    @abstractmethod
    async def post(self, content: str, metadata: Dict[str, Any] = None) -> str:
        """Post content, return post ID"""
        pass

    @abstractmethod
    async def reply(self, target_id: str, content: str) -> str:
        """Reply to target, return reply ID"""
        pass

    @abstractmethod
    async def react(self, target_id: str, reaction: str) -> bool:
        """React to target, return success"""
        pass

    @abstractmethod
    async def send_direct_message(self, user_id: str, content: str) -> bool:
        """Send DM, return success"""
        pass


class TwitterXAdapter(SocialPlatformAdapter):
    """Twitter/X platform integration"""

    def __init__(self):
        try:
            import tweepy
            self.tweepy = tweepy
        except ImportError:
            logger.warning("tweepy not installed, Twitter integration disabled")
            self.tweepy = None
        
        self.api = None
        self.client = None

    async def authenticate(self):
        """Authenticate with Twitter API v2"""
        if not self.tweepy:
            return False
        
        try:
            api_key = os.getenv("TWITTER_API_KEY")
            api_secret = os.getenv("TWITTER_API_SECRET")
            bearer_token = os.getenv("TWITTER_BEARER_TOKEN")
            
            if not all([api_key, api_secret, bearer_token]):
                logger.warning("Twitter credentials not configured")
                return False
            
            # Modern Twitter API v2
            self.client = self.tweepy.Client(bearer_token=bearer_token)
            logger.info("Twitter authentication successful")
            return True
        except Exception as e:
            logger.error(f"Twitter authentication failed: {e}")
            return False

    async def post(self, content: str, metadata: Dict[str, Any] = None) -> str:
        """Post tweet"""
        if not self.client:
            raise RuntimeError("Twitter client not authenticated")
        
        try:
            response = self.client.create_tweet(text=content)
            post_id = response.data['id']
            logger.info(f"Posted tweet: {post_id}")
            return post_id
        except Exception as e:
            logger.error(f"Failed to post tweet: {e}")
            raise

    async def reply(self, target_id: str, content: str) -> str:
        """Reply to tweet"""
        if not self.client:
            raise RuntimeError("Twitter client not authenticated")
        
        try:
            response = self.client.create_tweet(
                text=content,
                reply_settings="everyone",
                in_reply_to_tweet_id=target_id
            )
            reply_id = response.data['id']
            logger.info(f"Replied to tweet {target_id}: {reply_id}")
            return reply_id
        except Exception as e:
            logger.error(f"Failed to reply to tweet: {e}")
            raise

    async def react(self, target_id: str, reaction: str) -> bool:
        """Like tweet (limited reaction support)"""
        if not self.client:
            raise RuntimeError("Twitter client not authenticated")
        
        try:
            if reaction.lower() in ["like", "❤️", "👍"]:
                self.client.like(id=target_id)
                logger.info(f"Liked tweet {target_id}")
                return True
            else:
                logger.warning(f"Unsupported reaction on Twitter: {reaction}")
                return False
        except Exception as e:
            logger.error(f"Failed to react to tweet: {e}")
            raise

    async def send_direct_message(self, user_id: str, content: str) -> bool:
        """Send DM to user"""
        if not self.client:
            raise RuntimeError("Twitter client not authenticated")
        
        try:
            # Note: DMs require elevated access
            self.client.create_direct_message(
                participant_ids=[user_id],
                message_data={"text": content}
            )
            logger.info(f"Sent DM to {user_id}")
            return True
        except Exception as e:
            logger.error(f"Failed to send DM: {e}")
            raise


class TelegramAdapter(SocialPlatformAdapter):
    """Telegram platform integration"""

    def __init__(self):
        try:
            from telegram import Bot
            from telegram.error import TelegramError
            self.Bot = Bot
            self.TelegramError = TelegramError
        except ImportError:
            logger.warning("python-telegram-bot not installed, Telegram disabled")
            self.Bot = None
        
        self.bot = None

    async def authenticate(self):
        """Authenticate with Telegram Bot API"""
        if not self.Bot:
            return False
        
        try:
            token = os.getenv("TELEGRAM_BOT_TOKEN")
            if not token:
                logger.warning("Telegram token not configured")
                return False
            
            self.bot = self.Bot(token=token)
            logger.info("Telegram authentication successful")
            return True
        except Exception as e:
            logger.error(f"Telegram authentication failed: {e}")
            return False

    async def post(self, content: str, metadata: Dict[str, Any] = None) -> str:
        """Post to Telegram channel"""
        if not self.bot:
            raise RuntimeError("Telegram bot not authenticated")
        
        try:
            channel_id = metadata.get("channel_id") if metadata else None
            if not channel_id:
                raise ValueError("channel_id required in metadata")
            
            message = await self.bot.send_message(
                chat_id=channel_id,
                text=content,
                parse_mode="HTML" if "<" in content else None
            )
            logger.info(f"Posted to Telegram channel {channel_id}: {message.message_id}")
            return str(message.message_id)
        except Exception as e:
            logger.error(f"Failed to post to Telegram: {e}")
            raise

    async def reply(self, target_id: str, content: str) -> str:
        """Reply in Telegram thread"""
        if not self.bot:
            raise RuntimeError("Telegram bot not authenticated")
        
        try:
            # target_id format: "chat_id:message_id"
            chat_id, message_id = target_id.split(":")
            message = await self.bot.send_message(
                chat_id=chat_id,
                text=content,
                reply_to_message_id=int(message_id)
            )
            logger.info(f"Replied in Telegram: {message.message_id}")
            return str(message.message_id)
        except Exception as e:
            logger.error(f"Failed to reply on Telegram: {e}")
            raise

    async def react(self, target_id: str, reaction: str) -> bool:
        """React to Telegram message"""
        if not self.bot:
            raise RuntimeError("Telegram bot not authenticated")
        
        try:
            chat_id, message_id = target_id.split(":")
            await self.bot.set_message_reaction(
                chat_id=chat_id,
                message_id=int(message_id),
                reaction=reaction
            )
            logger.info(f"Reacted on Telegram: {reaction}")
            return True
        except Exception as e:
            logger.error(f"Failed to react on Telegram: {e}")
            raise

    async def send_direct_message(self, user_id: str, content: str) -> bool:
        """Send Telegram DM"""
        if not self.bot:
            raise RuntimeError("Telegram bot not authenticated")
        
        try:
            await self.bot.send_message(
                chat_id=user_id,
                text=content
            )
            logger.info(f"Sent Telegram DM to {user_id}")
            return True
        except Exception as e:
            logger.error(f"Failed to send Telegram DM: {e}")
            raise


class DiscordAdapter(SocialPlatformAdapter):
    """Discord platform integration"""

    def __init__(self):
        try:
            import discord
            self.discord = discord
        except ImportError:
            logger.warning("discord.py not installed, Discord disabled")
            self.discord = None
        
        self.bot = None

    async def authenticate(self):
        """Authenticate with Discord Bot"""
        if not self.discord:
            return False
        
        try:
            token = os.getenv("DISCORD_BOT_TOKEN")
            if not token:
                logger.warning("Discord token not configured")
                return False
            
            intents = self.discord.Intents.default()
            intents.message_content = True
            self.bot = self.discord.Client(intents=intents)
            
            await self.bot.login(token)
            logger.info("Discord authentication successful")
            return True
        except Exception as e:
            logger.error(f"Discord authentication failed: {e}")
            return False

    async def post(self, content: str, metadata: Dict[str, Any] = None) -> str:
        """Post to Discord channel"""
        if not self.bot:
            raise RuntimeError("Discord client not authenticated")
        
        try:
            channel_id = metadata.get("channel_id") if metadata else None
            if not channel_id:
                raise ValueError("channel_id required in metadata")
            
            channel = self.bot.get_channel(int(channel_id))
            if not channel:
                raise ValueError(f"Channel {channel_id} not found")
            
            message = await channel.send(content)
            logger.info(f"Posted to Discord channel {channel_id}: {message.id}")
            return str(message.id)
        except Exception as e:
            logger.error(f"Failed to post to Discord: {e}")
            raise

    async def reply(self, target_id: str, content: str) -> str:
        """Reply in Discord thread"""
        if not self.bot:
            raise RuntimeError("Discord client not authenticated")
        
        try:
            # target_id format: "channel_id:message_id"
            channel_id, message_id = target_id.split(":")
            
            channel = self.bot.get_channel(int(channel_id))
            if not channel:
                raise ValueError(f"Channel {channel_id} not found")
            
            target_message = await channel.fetch_message(int(message_id))
            reply = await target_message.reply(content)
            
            logger.info(f"Replied in Discord: {reply.id}")
            return str(reply.id)
        except Exception as e:
            logger.error(f"Failed to reply on Discord: {e}")
            raise

    async def react(self, target_id: str, reaction: str) -> bool:
        """React to Discord message"""
        if not self.bot:
            raise RuntimeError("Discord client not authenticated")
        
        try:
            channel_id, message_id = target_id.split(":")
            
            channel = self.bot.get_channel(int(channel_id))
            if not channel:
                raise ValueError(f"Channel {channel_id} not found")
            
            message = await channel.fetch_message(int(message_id))
            await message.add_reaction(reaction)
            
            logger.info(f"Reacted on Discord: {reaction}")
            return True
        except Exception as e:
            logger.error(f"Failed to react on Discord: {e}")
            raise

    async def send_direct_message(self, user_id: str, content: str) -> bool:
        """Send Discord DM"""
        if not self.bot:
            raise RuntimeError("Discord client not authenticated")
        
        try:
            user = await self.bot.fetch_user(int(user_id))
            await user.send(content)
            
            logger.info(f"Sent Discord DM to {user_id}")
            return True
        except Exception as e:
            logger.error(f"Failed to send Discord DM: {e}")
            raise


class SocialAgentsManager:
    """Manages social media agents and actions"""

    def __init__(self):
        self.platforms: Dict[str, SocialPlatformAdapter] = {
            "twitter": TwitterXAdapter(),
            "telegram": TelegramAdapter(),
            "discord": DiscordAdapter(),
        }
        
        self.pending_actions: List[SocialAction] = []
        self.action_handlers: Dict[str, Callable] = {}
        self.feature_flags: Dict[str, bool] = {}
        self._load_feature_flags()

    def _load_feature_flags(self):
        """Load feature flags for each platform"""
        self.feature_flags = {
            "twitter:enabled": os.getenv("TWITTER_AGENT_ENABLED", "true").lower() == "true",
            "twitter:posting": os.getenv("TWITTER_POSTING_ENABLED", "true").lower() == "true",
            "twitter:replies": os.getenv("TWITTER_REPLIES_ENABLED", "true").lower() == "true",
            "telegram:enabled": os.getenv("TELEGRAM_AGENT_ENABLED", "true").lower() == "true",
            "telegram:posting": os.getenv("TELEGRAM_POSTING_ENABLED", "true").lower() == "true",
            "discord:enabled": os.getenv("DISCORD_AGENT_ENABLED", "true").lower() == "true",
            "discord:posting": os.getenv("DISCORD_POSTING_ENABLED", "true").lower() == "true",
        }

    async def initialize_all(self):
        """Initialize and authenticate all platforms"""
        for platform_name, adapter in self.platforms.items():
            if not self.feature_flags.get(f"{platform_name}:enabled", True):
                logger.info(f"{platform_name} agent disabled via feature flag")
                continue
            
            try:
                success = await adapter.authenticate()
                if success:
                    logger.info(f"✓ {platform_name.capitalize()} social agent initialized")
                else:
                    logger.warning(f"✗ {platform_name.capitalize()} social agent initialization failed")
            except Exception as e:
                logger.error(f"Error initializing {platform_name}: {e}")

    async def queue_action(self, action: SocialAction):
        """Queue social media action"""
        self.pending_actions.append(action)
        logger.info(f"Queued {action.action_type} action on {action.platform}: {action.action_id}")

    async def execute_pending_actions(self):
        """Execute all pending social media actions"""
        while self.pending_actions:
            action = self.pending_actions.pop(0)
            
            try:
                await self._execute_action(action)
            except Exception as e:
                logger.error(f"Failed to execute action {action.action_id}: {e}")
                
                if action.retries < action.max_retries:
                    action.retries += 1
                    action.status = ActionStatus.RETRYING
                    self.pending_actions.append(action)
                else:
                    action.status = ActionStatus.FAILED
                    logger.error(f"Action {action.action_id} failed after {action.max_retries} retries")

    async def _execute_action(self, action: SocialAction):
        """Execute individual action"""
        adapter = self.platforms.get(action.platform)
        if not adapter:
            raise ValueError(f"Unknown platform: {action.platform}")

        if not self.feature_flags.get(f"{action.platform}:{action.action_type}", True):
            logger.warning(f"{action.action_type} disabled on {action.platform}")
            action.status = ActionStatus.FAILED
            return

        action.status = ActionStatus.EXECUTING
        
        if action.action_type == "post":
            await adapter.post(action.content, action.metadata)
        elif action.action_type == "reply":
            await adapter.reply(action.target, action.content)
        elif action.action_type == "react":
            await adapter.react(action.target, action.content)
        elif action.action_type == "direct_message":
            await adapter.send_direct_message(action.target, action.content)
        else:
            raise ValueError(f"Unknown action type: {action.action_type}")
        
        action.status = ActionStatus.SUCCESS

    def get_stats(self) -> Dict[str, Any]:
        """Get social agents statistics"""
        return {
            "pending_actions": len(self.pending_actions),
            "platforms_initialized": sum(1 for f in self.feature_flags.values() if f),
            "feature_flags": self.feature_flags,
            "timestamp": datetime.utcnow().isoformat(),
        }


# Singleton instance
_social_agents = None

def get_social_agents_manager() -> SocialAgentsManager:
    """Get or create social agents manager"""
    global _social_agents
    if _social_agents is None:
        _social_agents = SocialAgentsManager()
    return _social_agents


__all__ = [
    'SocialAgentsManager',
    'SocialAction',
    'ActionStatus',
    'TwitterXAdapter',
    'TelegramAdapter',
    'DiscordAdapter',
    'get_social_agents_manager',
]
