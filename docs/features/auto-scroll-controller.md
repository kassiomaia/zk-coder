# Auto-Scroll Controller

## Overview

The auto-scroll controller manages scroll behavior in chat conversations during AI response streaming. It keeps the chat window scrolled to the bottom as new content arrives, while still allowing users to pause and view older messages when needed.

## How It Works

When the AI starts streaming a response, the system automatically scrolls to show new content as it appears. This behavior continues throughout the streaming process without interruption.

Users can pause this automatic scrolling by clicking and dragging the scrollbar thumb upward. This signals that they want to view earlier messages in the conversation. Once they release the scrollbar, the view stays at their chosen position.

To resume automatic scrolling, users have two options. They can manually scroll back to the bottom of the conversation, or they can click the scroll-to-bottom button that appears when they are far from the latest messages.

## Key Behaviors

**During streaming:**
- New content automatically scrolls into view
- The scroll position stays locked to the bottom

**When user drags the scrollbar up:**
- Automatic scrolling pauses immediately
- The user can browse older messages freely
- New content continues arriving but does not disturb their view

**Resuming auto-scroll:**
- Scrolling back near the bottom restores automatic scrolling
- Clicking the scroll-to-bottom button jumps to the latest content and re-enables auto-scroll
- Sending a new message also re-enables auto-scroll

## Visual Indicators

A scroll-to-bottom button appears when the user has scrolled far enough from the latest messages. During streaming, this button pulses to indicate that new content is arriving below.

## Design Principles

The system distinguishes between user-initiated scrolling and programmatic scrolling. Programmatic scrolls (those triggered by the system) do not interfere with user preferences. Only when the user explicitly drags the scrollbar thumb does the system pause automatic scrolling.

This approach ensures that users can always view earlier parts of the conversation without fighting against incoming content, while maintaining the convenience of automatic scrolling for normal usage.
