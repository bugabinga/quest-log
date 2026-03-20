---
author: Oliver Jan Krylow <oliver@bugabinga.net>
date: 2026-03-19
status: done
tags: [spec, meta]
---

# Spec Zero

This is a specification of `specs`.

A file-based system of persistent prompts, that encode human intent to AI agents
over time.

Specs are Markdown files with YAML frontmatter. While the YAML has a strict
schema, the Markdown part is a relatively free form prompt text. All spec files
live in `documentation/specs` in arbitrary subfolders (or flat) and form an
undirected graph by linking to each other.

## Metadata in Frontmatter

The schema for the YAML frontmatter metadata is:

```yaml
author: author name and email
date: YYYY-MM-DD
status: some status
tags: [list of labels]
```

All fields are required, and only `tags` may be empty.

## Human Intent in Markdown Prompt Text

The whole point of a spec is to communicate human intent to AI Agents about
software projects. As such, specs are basically normal LLM prompts but:

- are persistent over time
- have metadata
- are carefully reviewed by humans and AIs

That means we expect specs to be written and maintained by humans (with possible
assistance by AI) and read many times by various agent systems.

The scope of specs are anything about a software project, that a human cares
about and values. That includes definitions of undesired state. By contrast,
anything not specified by specs, is up to the interpretation of AI's.

Specs are expected to be re-read by AI's many times in attempts to align the
software with the specification.

### Recommendations for a good spec

- use bullet points, it helps in keeping it concise and short
- not only define positive state (wishes, features, desired behaviour), but also
  negative space (things that are forbidden, should never happen, are undesired)
- use references and keep them up to date

### References to other specs

3 types of references are supported:

Markdown inline links:

```md
This is [inline link](/some-file.md#some-heading). This is an internal
[anchor link](#heading).
```

Markdown reference links:

```md
See [reference].

[reference]: /url "Title"
```

And Wiki-Links:

```md
Link to [[another-note]]. Link to [[another-notes#heading]]. Internal link to
[[#a-heading]].
```

While all 3 types can be used for any kind of reference, the preferred mapping
is:

- external URLs -> reference link
- internal ref to spec -> wiki link
- anything else -> inline link

## File Structure for Specs

Spec files are placed in `./documentation/specs` in the local project root. The
sub-folder hierarchy in this folder is arbitrary and may be organized by
whatever system makes sense for the project.

Spec files are markdown files with a strict naming pattern: `<XXXX>-<title>.md`.

**XXXX**: 4 globally unique characters (`[a-zA-Z0-9]{4}`).

**title**: a short and sluggified version of the spec title (e.g. "Spec Zero"
becomes "spec-zero") is recommended, but may be anything.

The title is only meant to be a reminder about the long title. But it is
imperative that the `<XXXX>` 4 characters are globally unique, such that specs
can be referenced unambiguously across the entire project regardless of folder
structure.

Example spec file structures:

Flat:

```
-- ./documentation/specs
-------------------------- a88t-general-idea.md
-------------------------- 94yt-main-view.md
-------------------------- 123d-login-system.md
-------------------------- 11hf-security.md
-------------------------- 04n4-visual-design-and-language.md
-------------------------- dfjr-the-send-button.md
```

Tree:

```
-- ./documentation/specs
-------------------------- general-idea/
---------------------------- A001.md
---------------------------- A002.md
-------------------------- main-view/
---------------------------- B001.md
---------------------------- B002.md
-------------------------- login-system/
---------------------------- C001.md
---------------------------- C002.md
---------------------------- C003.md
---------------------------- C004.md
-------------------------- security/
---------------------------- D001.md
-------------------------- visual-design-and-language/
---------------------------- E001.md
---------------------------- E002.md
-------------------------- the-send-button/
---------------------------- F001.md
```

Or the files may be entirely unstructured. It is up to the human to decide a
structure and for the AI's to interpret it.
