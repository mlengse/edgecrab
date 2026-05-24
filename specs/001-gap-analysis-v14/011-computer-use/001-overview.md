# 011 — Provider-Agnostic Computer Use (via `cua-driver`)

**Tier:** A | **Impact:** 4 | **Value-per-Effort:** 3 | **Risk:** 4
**Primitive moved:** Reach (qualitatively new tasks)

## Why It Matters (First Principles)

"Computer use" — screen capture + click/type/scroll primitives driven by
a multimodal model — unlocks a category of work no purely-text agent can
do: filling visual web forms with anti-bot defenses, controlling
desktop apps without APIs, navigating legacy software, automating
GUI-only IT admin tasks.

Anthropic's Claude has a proprietary "computer use" tool family; OpenAI
has a similar `computer_use_preview`. Hermes v0.14 generalised this by
adopting **`cua-driver`** (a provider-agnostic computer-use abstraction)
so the same `computer_use` tool works against Claude, GPT-4 Vision,
Gemini, and local screen-capture backends. The tool isn't tied to a
single provider's wire format.

## The Gap

EdgeCrab has:
- `vision` tool (image analysis only).
- `browser` tool (CDP / headless Chrome) — useful but constrained to a
  browser.

EdgeCrab does not have:
- A `computer_use` tool that screenshots the *desktop* and emits
  click/type/scroll primitives that translate to native OS actions.
- A provider-agnostic abstraction; the closest equivalents (Anthropic's
  built-in computer-use beta, OpenAI's preview) are provider-coupled.

## What EdgeCrab Gets Wrong Today

For any task outside a browser — say, "fill the PDF form in Preview app
and save it to the desktop" — there is no path. Users either drop to the
shell (limited) or open Claude Desktop directly.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
