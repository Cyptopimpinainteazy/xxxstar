module X3VM-SYNTAX
  imports INT

  syntax Instr ::= "NOOP" | "HALT"
  syntax Program ::= List{Instr, ";"}
endmodule

module X3VM-SPEC
  imports X3VM-SYNTAX

  configuration <k> $PGM:Program </k>

  rule <k> NOOP ; REST => REST ...</k>
  rule <k> HALT ; _ => .Program ...</k>

endmodule
