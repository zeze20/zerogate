-------------------------- MODULE FrameOwnership --------------------------
(***************************************************************************)
(* Formal specification of the MR10 frame ownership state machine.        *)
(*                                                                         *)
(* This models the abstract ownership lifecycle of UMEM frames managed by  *)
(* `FramePool` in zerogate-agent/src/frame.rs. Every frame is in exactly   *)
(* one ownership state at any time and may only move through legal          *)
(* lifecycle transitions.                                                   *)
(*                                                                         *)
(* Scope: this model checks ownership-lifecycle correctness only. It does  *)
(* NOT model the Linux kernel, the eBPF verifier, the AF_XDP runtime,       *)
(* NIC/DMA hardware, the Rust compiler, or the Rust implementation          *)
(* line-by-line. See docs/FRAME_OWNERSHIP.md for the full mapping and       *)
(* scope statement.                                                         *)
(*                                                                         *)
(* Free-list abstraction: the Rust implementation backs the free list with *)
(* a VecDeque (stack/queue). This model abstracts it as a SET of free       *)
(* frames. Allocation order is intentionally abstracted away; set-based     *)
(* reasoning is sufficient for ownership correctness (no duplicates, no     *)
(* non-Free frame in the free list, free set matches Free state).          *)
(***************************************************************************)

EXTENDS Naturals

CONSTANT N

ASSUME N \in Nat /\ N > 0

Frames == 0 .. (N - 1)

FrameStates ==
    {"Free", "InFill", "Kernel", "Rx", "User", "Tx", "Completion"}

VARIABLES
    state,   \* state[f] : current ownership state of frame f
    free     \* set of frames currently in the Free state

vars == << state, free >>

(***************************************************************************)
(* Legal transition relation. Mirrors FrameState::can_transition_to.       *)
(***************************************************************************)
CanTransition(from, to) ==
    \/ /\ from = "Free"       /\ to = "InFill"
    \/ /\ from = "InFill"     /\ to = "Kernel"
    \/ /\ from = "Kernel"     /\ to = "Rx"
    \/ /\ from = "Rx"         /\ to = "User"
    \/ /\ from = "User"       /\ to = "InFill"
    \/ /\ from = "User"       /\ to = "Tx"
    \/ /\ from = "Tx"         /\ to = "Completion"
    \/ /\ from = "Completion" /\ to = "Free"

(***************************************************************************)
(* Initial state: all frames Free, the free set is every frame.            *)
(***************************************************************************)
Init ==
    /\ state = [f \in Frames |-> "Free"]
    /\ free = Frames

(***************************************************************************)
(* Actions. Each corresponds to exactly one FramePool transition method.   *)
(***************************************************************************)

\* allocate_for_fill: Free -> InFill, frame removed from free set.
AllocateForFill ==
    \E f \in free :
        /\ state[f] = "Free"
        /\ state' = [state EXCEPT ![f] = "InFill"]
        /\ free' = free \ {f}

\* mark_kernel_owned: InFill -> Kernel.
MarkKernelOwned ==
    \E f \in Frames :
        /\ state[f] = "InFill"
        /\ state' = [state EXCEPT ![f] = "Kernel"]
        /\ free' = free

\* mark_rx: Kernel -> Rx.
MarkRx ==
    \E f \in Frames :
        /\ state[f] = "Kernel"
        /\ state' = [state EXCEPT ![f] = "Rx"]
        /\ free' = free

\* acquire_user: Rx -> User.
AcquireUser ==
    \E f \in Frames :
        /\ state[f] = "Rx"
        /\ state' = [state EXCEPT ![f] = "User"]
        /\ free' = free

\* recycle_to_fill: User -> InFill. Does NOT add to free (InFill is not Free).
RecycleToFill ==
    \E f \in Frames :
        /\ state[f] = "User"
        /\ state' = [state EXCEPT ![f] = "InFill"]
        /\ free' = free

\* submit_tx: User -> Tx.
SubmitTx ==
    \E f \in Frames :
        /\ state[f] = "User"
        /\ state' = [state EXCEPT ![f] = "Tx"]
        /\ free' = free

\* complete_tx: Tx -> Completion.
CompleteTx ==
    \E f \in Frames :
        /\ state[f] = "Tx"
        /\ state' = [state EXCEPT ![f] = "Completion"]
        /\ free' = free

\* release_completion: Completion -> Free, frame added back to free set.
ReleaseCompletion ==
    \E f \in Frames :
        /\ state[f] = "Completion"
        /\ state' = [state EXCEPT ![f] = "Free"]
        /\ free' = free \cup {f}

Next ==
    \/ AllocateForFill
    \/ MarkKernelOwned
    \/ MarkRx
    \/ AcquireUser
    \/ RecycleToFill
    \/ SubmitTx
    \/ CompleteTx
    \/ ReleaseCompletion

Spec == Init /\ [][Next]_vars

(***************************************************************************)
(* Invariants.                                                             *)
(***************************************************************************)

TypeOK ==
    /\ state \in [Frames -> FrameStates]
    /\ free \subseteq Frames

\* Because `state` is a total function Frames -> FrameStates, every frame
\* has exactly one well-defined state.
FrameInExactlyOneState ==
    \A f \in Frames : state[f] \in FrameStates

FreeListOnlyFree ==
    \A f \in free : state[f] = "Free"

\* Stronger: the free set is exactly the set of Free frames. Catches a
\* missing free frame, a non-Free frame in free, or an extra frame in free.
FreeListMatchesState ==
    free = {f \in Frames : state[f] = "Free"}

NonFreeNotInFree ==
    \A f \in Frames : state[f] # "Free" => f \notin free

TxNotFreeBeforeCompletion ==
    \A f \in Frames : state[f] = "Tx" => f \notin free

UserNotFree ==
    \A f \in Frames : state[f] = "User" => f \notin free

RxNotFree ==
    \A f \in Frames : state[f] = "Rx" => f \notin free

InFillNotFree ==
    \A f \in Frames : state[f] = "InFill" => f \notin free

KernelNotFree ==
    \A f \in Frames : state[f] = "Kernel" => f \notin free

CompletionNotFree ==
    \A f \in Frames : state[f] = "Completion" => f \notin free

OwnershipConsistent ==
    /\ FreeListOnlyFree
    /\ FreeListMatchesState
    /\ NonFreeNotInFree
    /\ TxNotFreeBeforeCompletion
    /\ UserNotFree
    /\ RxNotFree
    /\ InFillNotFree
    /\ KernelNotFree
    /\ CompletionNotFree

=============================================================================
