---
name: code-reviewer
description:
    Use this skill to review code. It supports both local changes and remove Pull Requests.
---

# Code Reviewer

This skill guides the agent in conducting code reviews through the codebase.


## Project Overview

This a tauri application that enables users to have a agent workspace on their local machine.

- **Chat**: Chat is the main interface for the user to interact with the agent.
- **Rust**: Tauri provide a Rust backend for the application.

## Code Locations

| Location                                             | Description         |
|------------------------------------------------------|---------------------|
| [`./src-tauri/src`](./src-tauri/src)                 | Tauri backend       |
| [`./src-ui/src`](./src-ui/src)                       | Tauri frontend      |
--

## Frontend

- Svelte
- CSS
- Typescript


## Workflow

    
1) The agent can ask to review a specific file.
2) The agent can ask to review the changes on `uncommitted` files.

## Reviewing Code (Features)

- **Quality**: The agent can ask to review the quality of the code.
- **Security**: The agent can ask to review the security of the code.
- **Durability**: The agent can ask to review the durability of the code.
- **Readability**: The agent can ask to review the readability of the code.
- **Maintainability**: The agent can ask to review the maintainability of the code.
- **Reusable**: The agent can ask to review the reusability of the code.
- **Functionality**: The agent can ask to review the functionality of the code.

## Code Review Document Template

```markdown
# Code Review Summary

DATE TIME: <date_time>
AGENT: <agent_name>

-

{{GENERAL_DESCRIPTION}}


COMMENTS: {{Number of comments}}
ISSUES: {{Number of issues}}
ASK QUESTIONS: {{Number of questions}}

-

<for_each_comment>
--
[HIGH|LOW|MEDIUM] - [comment_number] - <comment_title>
    [comment_description]

    Files:
    <for_each_file>
        - <file_name>:<line_number>:<column_number>
    </for_each_file>

    [Suggested Fix]: <fix_text>
--
</for_each_comment>

<review_summary>
# Summary

    <for_each_summary_field>
      - <summary_field_name>: <summary_field_value>
    </for_each_summary_field>
</review_summary>

-

STATUS: <status>
SIGNATURE: <unique_id>:<agent_name>:<date_time>:<status>
```

## Outcome

Once the agent has finished reviewing the code, the agent will provide a summary of the review.
Then the agent will write all comments to the codebase.


Write all comments and save the reviews on `./docs/reviews/<agent_name>/<unique_id>.md`
