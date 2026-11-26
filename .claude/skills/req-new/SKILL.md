---
name: req-new
description: Writes a new SoftwareFactory requirement in requirements/ with the next free ID, then checks it does not contradict existing ones. Use when the user wants to add, write or describe a new requirement or feature.
argument-hint: <free-form description of the feature>
---

Need expressed by the user: $ARGUMENTS

1. **Find the next free ID.** Take the highest `REQ-NNN` number used among:
   - the files on master: `git ls-tree --name-only master requirements/`;
   - the files in the `requirements/` folder;
   - the branches: `git branch --list "REQ-*"`.

   The next ID is that number + 1, on 3 digits. If there is none, it is `REQ-001`.
2. **Write the requirement** in English, following `requirements/_TEMPLATE.md`:
   - a short title;
   - a precise description that covers edge cases;
   - a "How to test" section made of verifiable steps, each with a measurable expected result.

   If the need is too vague to write verifiable tests, first ask the user the necessary questions.
3. **Save** the file as `requirements/<ID>-<short-kebab-title>.md`. Do not commit it: the implementer will commit it on its branch.
4. **Check consistency** by delegating to the `req-checker` subagent, with the file path.
5. **Report** to the user:
   - if the verdict is `OK`: the file path and the command to start the chain:
     `./scripts/factory.sh run requirements/<file>.md`
   - if the verdict is `CONFLICT`: the contradictions found, and a suggested rewording.
