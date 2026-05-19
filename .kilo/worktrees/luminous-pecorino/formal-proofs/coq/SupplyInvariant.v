Require Import Coq.Arith.Arith.

Record State := {
  total_supply : nat;
}.

Inductive Tx :=
| Transfer (amount : nat)
| Mint (amount : nat)
| Burn (amount : nat).

Definition apply_tx (s : State) (tx : Tx) : option State :=
  match tx with
  | Transfer _ => Some s
  | Mint amt => Some {| total_supply := s.(total_supply) + amt |}
  | Burn amt =>
      if amt <=? s.(total_supply)
      then Some {| total_supply := s.(total_supply) - amt |}
      else None
  end.

Theorem supply_conservation :
  forall s s' tx,
    apply_tx s tx = Some s' ->
    match tx with
    | Transfer _ => total_supply s' = total_supply s
    | Mint amt => total_supply s' = total_supply s + amt
    | Burn amt => total_supply s' = total_supply s - amt
    end.
Proof.
  intros s s' tx H.
  destruct tx as [amt | amt | amt]; simpl in H.
  - inversion H; reflexivity.
  - inversion H; reflexivity.
  - destruct (amt <=? total_supply s) eqn:HB; inversion H; reflexivity.
Qed.
