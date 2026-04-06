---
name: Always quote CSV fields
description: When generating CSV files, always quote all fields to handle spaces and special characters
type: feedback
---

Always use proper quoting in CSV files — quote all fields, not just ones that happen to contain spaces.

**Why:** Unquoted fields with spaces or commas break CSV parsing. Inconsistent quoting (some quoted, some not) is sloppy.

**How to apply:** When writing CSV in Python, use `csv.QUOTE_ALL`. When generating CSV inline, wrap every field in double quotes.
