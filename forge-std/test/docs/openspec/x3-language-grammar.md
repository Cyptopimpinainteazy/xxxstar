# X3 Language Grammar

## Overview

X3 is stitched around a deterministic, Pratt-style expression parser with a block-oriented statement layer designed for agents, atomic windows, and financial primitives. This document captures the shape of the grammar that every parser implementation on X3 Chain must obey. Refer to these sections when generating ASTs, writing compiler passes, or onboarding AI agents that mutate the language.

## Statement Grammar (EBNF)

```
program        ::= { item } EOF ;
item           ::= function_decl | global_let | agent_decl | import_decl | const_decl ;
function_decl  ::= "fn" identifier "(" [ param_list ] ")" [ "->" type ] block ;
global_let     ::= "let" identifier [ ":" type ] "=" expression ";" ;
block          ::= "{" { stmt } "}" ;

stmt           ::= let_stmt
+                | expr_stmt
+                | return_stmt
+                | atomic_block
+                | if_stmt
+                | while_stmt
+                | loop_stmt
+                | for_stmt
+                | emit_stmt
+                ;

let_stmt       ::= "let" "mut"? identifier [ ":" type ] "=" expression ";" ;
return_stmt    ::= "return" expression? ";" ;
expr_stmt      ::= expression ";" ;
atomic_block   ::= "atomic" "(" expression ")" block | "atomic" block ;
if_stmt        ::= "if" "(" expression ")" block [ "else" ( block | if_stmt ) ] ;
while_stmt     ::= "while" "(" expression ")" block ;
loop_stmt      ::= "loop" block ;
for_stmt       ::= "for" "(" ( let_stmt | expr_stmt | ";" ) expression? ";" expression? ")" block ;
emit_stmt      ::= "emit" identifier "(" [ expression { "," expression } ] ")" ";" ;
```

## Expression Grammar (Pratt-friendly)

```
expression     ::= assignment ;
assignment     ::= conditional { "=" assignment } ;
conditional    ::= logical_or [ "?" expression ":" expression ] ;
logical_or     ::= logical_and { "||" logical_and } ;
logical_and    ::= bitwise_or { "&&" bitwise_or } ;
bitwise_or     ::= bitwise_xor { "|" bitwise_xor } ;
bitwise_xor    ::= bitwise_and { "^" bitwise_and } ;
bitwise_and    ::= equality { "&" equality } ;
equality       ::= comparison { ( "==" | "!=" ) comparison } ;
comparison     ::= shift { ( "<" | ">" | "<=" | ">=" ) shift } ;
shift          ::= additive { ( "<<" | ">>" ) additive } ;
additive       ::= multiplicative { ( "+" | "-" ) multiplicative } ;
multiplicative ::= unary { ( "*" | "/" | "%" ) unary } ;
unary          ::= ( "!" | "-" | "+" ) unary | call ;
call           ::= primary { call_suffix } ;
call_suffix    ::= "(" [ arg_list ] ")" | "." identifier | "[" expression "]" ;
arg_list       ::= expression { "," expression } ;
primary        ::= literal | identifier | "(" expression ")" | array_literal | struct_literal ;
```

## Operator Precedence Table (highest binds tighter)

| Precedence | Operators                              | Notes             |
| ---------- | -------------------------------------- | ----------------- |
| 120        | Function calls, indexing, field access | Postfix operators |
| 110        | Prefix unary (`!`, `-`, `+`)           | Right-associative |
| 100        | Multiplicative (`*`, `/`, `%`)         | Left-associative  |
| 90         | Additive (`+`, `-`)                    | Left-associative  |
| 80         | Shift (`<<`, `>>`)                     | Left-associative  |
| 70         | Comparison (`<`, `>`, `<=`, `>=`)      | Left-associative  |
| 60         | Equality (`==`, `!=`)                  | Left-associative  |
| 50         | Bitwise AND (`&`)                      | Left-associative  |
| 45         | Bitwise XOR (`^`)                      | Left-associative  |
| 40         | Bitwise OR (`                          | `)                | Left-associative |
| 35         | Logical AND (`&&`)                     | Left-associative  |
| 30         | Logical OR (`                          |                   | `)               | Left-associative |
| 25         | Conditional (`?:`)                     | Right-associative |
| 20         | Assignment (`=`, `+=`, `-=`)           | Right-associative |
| 10         | Comma / argument separator             | Lowest precedence |

## Parsing Guidance

- The parser must preserve deterministic ordering; statement lists, arguments, and declarations follow source order exactly.
- Every expression is composed with Pratt parsing so the precedence table above matches the binary operators emitted in the AST.
- Atomic blocks denote commit/rollback windows; if `atomic` is followed by parentheses, the enclosed expression becomes the metadata guard for the block.
- Semicolons terminate statements unless a block explicitly absorbs them (e.g., the `if` and `loop` bodies).
- Use this grammar as the reference when writing parser tests, compiler passes, or AI prompts for mutate/evolve steps.