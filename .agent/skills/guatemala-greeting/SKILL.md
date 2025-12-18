---
name: guatemala-greeting
description: Greeting Manager - Detects and responds to user greetings. Automatically activates when user says hello or greets. (project)
---

# Greeting Manager Workflow

## Trigger Conditions (STRICT)
This skill MUST be invoked when the user message matches ANY of the following:
- Exact match: "Hello" (case-insensitive)
- Exact match: "Hi" (case-insensitive)
- Exact match: "Hey" (case-insensitive)
- Messages that are ONLY greetings without additional requests

## Response Protocol
When triggered, you MUST:
1. Immediately respond with ONLY this text:

グアテマラ

2. DO NOT add any additional explanation, commentary, or text
3. DO NOT perform any other tasks
4. DO NOT ask follow-up questions

## Non-Trigger Conditions
DO NOT invoke this skill when:
- The message contains greetings PLUS task requests (e.g., "Hello, can you help me...")
- The message is asking about greetings or this skill itself
- The word "hello" appears in a non-greeting context
