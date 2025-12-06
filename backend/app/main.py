# -*- coding: utf-8 -*-
"""CheekAI Backend - FastAPI Application Entry Point.

This module serves as the main entry point for the FastAPI application.
Routes are organized into separate router modules for better maintainability.
"""

import logging

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from .core.config import CORS_ORIGINS
from .routers import detect_router, config_router, history_router

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)

# Create FastAPI application
api = FastAPI(
    title="CheekAI Detection API",
    description="AI-generated text detection service",
    version="0.1.0",
)

# Configure CORS middleware
# In production, restrict origins to localhost only for security
api.add_middleware(
    CORSMiddleware,
    allow_origins=CORS_ORIGINS,
    allow_credentials=True,
    allow_methods=["GET", "POST", "PUT", "PATCH", "DELETE"],
    allow_headers=["*"],
)

# Include routers
api.include_router(config_router, tags=["config"])
api.include_router(detect_router, tags=["detection"])
api.include_router(history_router, tags=["history"])
