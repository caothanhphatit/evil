# Game Design Specifications

This directory contains implementation-facing feature specifications for the
clean-room rebuild. The documents describe observable product behavior; they do
not override runtime evidence, generated content catalogs, or server authority
rules.

## Evidence labels

- **Package-confirmed**: recovered from the supplied `1.411` package, generated
  QuickSheet catalogs, serialized Unity content, or controlled runtime capture.
- **Official-reference**: documented by the publisher's public help center. The
  reference may describe a newer live version than the supplied package.
- **User-raw**: supplied manually and preserved for review; it may be incomplete
  or inaccurate.
- **Unresolved**: not safe to implement as original behavior without more
  evidence.

## Authoring workflow

1. Keep raw tables or notes traceable to their source.
2. Normalize terminology and prose in the feature specification.
3. Compare raw claims against package evidence and official references.
4. Record disagreements explicitly; never silently choose the convenient value.
5. Implement RNG, economy, progression, or rewards only from a versioned,
   reviewed rule set on the authoritative server.

Current specifications:

- [Hunter system](hunter-system.md)
- [Hunter lifecycle and command system](hunter-command-system.md)
- [Hunter personality system](hunter-personality-system.md)
- [Hunter skills: official reference](hunter-skills-official-reference.md)

Supporting migration evidence:

- [Zendesk Hunter skill catalog](../migration/zendesk-hunter-skill-catalog-v1.md)
