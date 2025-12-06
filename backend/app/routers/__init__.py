# -*- coding: utf-8 -*-
"""Routers module - API endpoints."""

from .detect import router as detect_router
from .config import router as config_router
from .history import router as history_router

__all__ = ["detect_router", "config_router", "history_router"]
