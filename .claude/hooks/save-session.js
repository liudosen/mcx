#!/usr/bin/env node
/**
 * Session Memory Saver
 * Runs at session end to save learned patterns and decisions
 */

const fs = require('fs');
const path = require('path');

const SESSION_FILE = path.join(__dirname, '../context/last-session.md');
const MEMORY_DIR = path.join(__dirname, '../context');

// Ensure directory exists
if (!fs.existsSync(MEMORY_DIR)) {
    fs.mkdirSync(MEMORY_DIR, { recursive: true });
}

// Get session info from environment or use defaults
const date = new Date().toISOString().slice(0, 16).replace('T', ' ');

const sessionTemplate = `# Last Session: ${date}

<!-- Auto-generated session summary - edit this file to preserve learnings -->

## Work Completed
-

## Key Decisions
-

## Patterns Learned
-

## Issues Resolved
-
`;

if (!fs.existsSync(SESSION_FILE)) {
    fs.writeFileSync(SESSION_FILE, sessionTemplate);
}

console.log('Session memory saved to:', SESSION_FILE);
