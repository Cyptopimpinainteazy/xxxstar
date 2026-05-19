## Requirements

### Requirement: X3 parser grammar reference
The system MUST publish `openspec/x3-language-grammar.md` as the canonical description of the X3 grammar. The document shall describe every statement production, function signature, `let` binding, and the Pratt-style expression rules with their precedence tiers so that parser implementers and AI agents share exactly the same surface syntax reference.

#### Scenario: Grammar review
- **WHEN** a compiler engineer, validator developer, or AI prompt reads the grammar doc
- **THEN** they can cite the exact production rule and precedence level for any expression or statement.

### Requirement: Parser regression coverage
The parser suite located in `crates/x3-parser` SHALL include tests that parse functions with `let` bindings, return statements, and nested call expressions which anchor the Pratt precedence table. These tests act as regression guards whenever the parser or lexer evolves.

#### Scenario: Parser regression guard
- **WHEN** `cargo test -p x3-parser` runs after code changes
- **THEN** the tests parse the reference function and fail if the AST no longer matches the documented grammar.