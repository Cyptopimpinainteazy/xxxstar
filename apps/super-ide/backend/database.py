import os
from datetime import datetime, timezone
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker, AsyncSession
from sqlalchemy.orm import DeclarativeBase
from sqlalchemy import Column, Integer, String, Text, DateTime, Float, Boolean

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
DATABASE_URL = f"sqlite+aiosqlite:///{os.path.join(BASE_DIR, 'super_ide.db')}"

engine = create_async_engine(DATABASE_URL, echo=False)
async_session = async_sessionmaker(engine, expire_on_commit=False)


class Base(DeclarativeBase):
    pass


class Block(Base):
    __tablename__ = "blocks"
    id = Column(Integer, primary_key=True, autoincrement=True)
    number = Column(Integer, unique=True, nullable=False)
    hash = Column(String(66), unique=True, nullable=False)
    timestamp = Column(DateTime, default=lambda: datetime.now(timezone.utc))
    tx_count = Column(Integer, default=0)
    producer = Column(String(42), nullable=True)


class Transaction(Base):
    __tablename__ = "transactions"
    id = Column(Integer, primary_key=True, autoincrement=True)
    hash = Column(String(66), unique=True, nullable=False)
    block_number = Column(Integer, nullable=False)
    from_address = Column(String(42), nullable=False)
    to_address = Column(String(42), nullable=True)
    value = Column(String(100), default="0")
    data = Column(Text, nullable=True)
    gas_limit = Column(Integer, nullable=True)
    gas_price = Column(String(100), nullable=True)
    status = Column(String(20), default="pending")
    timestamp = Column(DateTime, default=lambda: datetime.now(timezone.utc))


class Account(Base):
    __tablename__ = "accounts"
    id = Column(Integer, primary_key=True, autoincrement=True)
    address = Column(String(42), unique=True, nullable=False)
    public_key = Column(String(130), nullable=True)
    key_type = Column(String(20), default="ed25519")
    balance = Column(String(100), default="0")
    nonce = Column(Integer, default=0)
    label = Column(String(100), nullable=True)
    network = Column(String(20), default="local")
    created_at = Column(DateTime, default=lambda: datetime.now(timezone.utc))


class Contract(Base):
    __tablename__ = "contracts"
    id = Column(Integer, primary_key=True, autoincrement=True)
    address = Column(String(42), unique=True, nullable=False)
    name = Column(String(100), nullable=True)
    abi = Column(Text, nullable=True)
    bytecode = Column(Text, nullable=True)
    source_path = Column(String(500), nullable=True)
    compiler = Column(String(20), default="solidity")
    owner = Column(String(42), nullable=True)
    deployed_at = Column(DateTime, default=lambda: datetime.now(timezone.utc))
    verified = Column(Boolean, default=False)
    tx_hash = Column(String(66), nullable=True)


class Project(Base):
    __tablename__ = "projects"
    id = Column(Integer, primary_key=True, autoincrement=True)
    name = Column(String(100), nullable=False)
    path = Column(String(500), nullable=False, unique=True)
    template = Column(String(100), nullable=True)
    created_at = Column(DateTime, default=lambda: datetime.now(timezone.utc))


async def init_db():
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)


async def get_session() -> AsyncSession:
    async with async_session() as session:
        yield session
