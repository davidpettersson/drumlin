// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Keeps the exact text of the errors shown in `docs/src/reference/errors.md`.

const CASES: [(&str, &str); 58] = [
    ("component Scheduler\ncomponent Worker\nScheduelr --> Worker", "3:1: error: unknown element `Scheduelr`; did you mean `Scheduler`?"),
    ("component Worker\ncomponent Worker", "2:11: error: `Worker` is already declared"),
    ("component Outer {\n  component Inner", "2:1: error: missing `}` for `Outer`"),
    ("component Lonely\n}", "2:1: error: unmatched `}`"),
    ("provides IFoo", "1:1: error: `provides` and `requires` belong inside a component"),
    ("component A { port data }\ncomponent B { port data }\ncomponent C\ndata --> C", "4:1: error: port `data` is ambiguous; write `<component>.data`"),
    ("diagram package\npackage A\nA ..> B", "3:7: error: unknown element `B`; did you mean `A`?"),
    ("diagram package\nclass A {\n}", "2:1: error: only a package can have a `{ }` body"),
    ("diagram package\npackage Outer { class Inner }\nOuter +-- Inner", "3:1: error: `Inner` is drawn inside `Outer`, so containment needs no line; declare `Inner` outside the `Outer` block"),
    ("diagram state\nlayout dot\n[*] --> A", "2:8: error: unknown layout engine `dot`; the engines are `longest-path`, `network-simplex`"),
    ("component A\nlayout longest-pth", "2:8: error: unknown layout engine `longest-pth`; did you mean `longest-path`?"),
    ("diagram package\npackage A\nlayout", "3:1: error: expected `layout <engine>`; the engines are `longest-path`, `network-simplex`"),
    ("diagram sequence\nlayout longest-path\nparticipant A", "2:1: error: sequence diagrams have their own layout; layout engines apply to component, package and state diagrams"),
    ("diagram activity\nlayout longest-path\nstart\nstop", "2:1: error: activity diagrams have their own layout; layout engines apply to component, package and state diagrams"),
    ("diagram sequence\nstyle sketch\nparticipant A", "2:7: error: unknown style `sketch`; the styles are `pens`, `plain`, `lettering`"),
    ("component A\nstyle pen", "2:7: error: unknown style `pen`; did you mean `pens`?"),
    ("diagram activity\nstyle\nstart\nstop", "2:1: error: expected `style <name>`; the styles are `pens`, `plain`, `lettering`"),
    ("diagram timing \"Handshake\"", "1:1: error: diagram kind `timing` is not implemented; this prototype supports `component`, `package`, `sequence`, `state`, `activity`"),
    ("diagram sequence\nparticipant Server\nServre -> Server : ping", "3:1: error: unknown participant `Servre`; did you mean `Server`?; declare it with `participant` before its first use"),
    ("diagram sequence\nparticipant A\nreturn done", "3:1: error: `return` belongs inside a message block"),
    ("diagram sequence\nparticipant A\nparticipant B\nopt \"x\" {\n  A -> B : y\n} else {\n  B -> A : z\n}", "6:3: error: `else` must follow the `}` of an `alt`"),
    ("diagram sequence\nparticipant A\nA ->] : gone {\n}", "3:1: error: a lost message has no receiver to run a `{ }` block"),
    ("diagram sequence\nparticipant A\ndestroy A\nA -> A : ghost", "4:1: error: `A` was already destroyed"),
    ("diagram sequence\nparticipant A\nparticipant A", "3:13: error: `A` is already declared"),
    ("diagram sequence\nparticipant A\nparticipant B\nloop {\n  A -> B : y", "5:1: error: missing `}` for `loop`"),
    ("diagram sequence\nparticipant A\nparticipant B\nA -> B : y {\n}\n}", "6:1: error: unmatched `}`"),
    ("diagram sequence\nparticipant A\nactro B", "3:7: error: expected an arrow after `actro`, found `B`; did you mean `actor`?"),
    ("diagram sequence\nparticipant A\nopt \"x\" {\n}", "3:1: error: the `opt` fragment is empty"),
    ("diagram sequence\nA -> A", "2:1: error: unknown participant `A`; declare it with `participant` before its first use"),
    ("diagram state\nstate A\nstate A", "3:7: error: `A` is already declared"),
    ("diagram state\nstate Idle\n[*] --> Idle\nIdle --> Idel : wait", "4:10: error: unknown state `Idel`; did you mean `Idle`? declare it with `state Idel` if intended"),
    ("diagram state\nfork f\nA --> f\nf --> B", "2:6: error: fork `f` needs exactly one incoming and at least two outgoing transitions"),
    ("diagram state\njoin j\nA --> j\nj --> B", "2:6: error: join `j` needs at least two incoming transitions and exactly one outgoing"),
    ("diagram state\nchoice c\nA --> c\nc --> B : go", "4:7: error: a transition leaving a pseudostate cannot have a trigger; use `[guard] / effect`"),
    ("diagram state\nfork f\nA --> f\nf --> B : [x]\nf --> C", "4:7: error: a fork or join segment cannot have a trigger or guard"),
    ("diagram state\nstate S {\n  [*] --> A : go\n}", "3:11: error: a nested initial pseudostate's transition must be unlabeled"),
    ("diagram state\n[*] --> A\n[*] --> B", "2:1: error: a region has one initial pseudostate, with one outgoing transition"),
    ("diagram state\nstate A {\n  region {\n    [*] --> B\n  }\n  region {\n    [*] --> C\n    C --> B\n  }\n}", "8:5: error: a transition cannot join two orthogonal regions of `A`"),
    ("diagram state\nA --> B : go [ok] then", "2:9: error: expected `/` before `then`; a label has the form `trigger [guard] / effect`"),
    ("diagram state\nstat A", "2:6: error: expected an arrow after `stat`, found `A`; did you mean `state`?"),
    ("diagram state\nA -> B", "2:3: error: a transition is written with `-->`"),
    ("diagram activity\nstart\nstop\naction \"Late\"", "4:1: error: this statement can never run: the flow already ended with `stop` on line 3; remove it, or move it before the end"),
    ("diagram activity\nstart\nfork {\n  action \"A\"\n} and {\n}\nstop", "5:3: error: this `fork` branch is empty; each branch needs a step, or remove the branch"),
    ("diagram activity\naction \"A\"\nstart", "3:1: error: an initial node has no incoming flow, so `start` must be the first statement"),
    ("diagram activity\nstart\nobject \"Order\"", "3:1: error: an initial node can only begin a control flow, so `object` cannot follow `start`; put an action or `accept` between them"),
    ("diagram activity\nnote: nothing yet", "2:1: error: a note attaches to the step before it, and this flow has no step yet"),
    ("diagram activity\nstart\nswitch \"x\" {\n  case \"a\" {\n    action \"A\"\n  }\n}\nstop", "3:1: error: a `switch` needs at least two `case` branches; use `if` for one"),
    ("diagram activity\nstart\nsplit", "3:1: error: `split` is not available; use `fork` for concurrent flows or `switch` for alternatives"),
    ("diagram activity\nstart\nelse {\n}", "3:1: error: `else` must follow the `}` of an `if`"),
    ("diagram activity\nstart\nrepeat {\n  action \"A\"\n}\nstop", "3:1: error: a `repeat` block needs `until \"condition\"` after its `}`"),
    ("diagram activity\nstart\nwhile \"x\" {\n  stop\n}\nstop", "3:1: error: the body of this `while` always ends the flow, so it can never loop; use `if`"),
    ("diagram activity\nstart\nif \"x\" {\n  action \"A\"\n}", "3:1: error: the flow continues after this `if` but nothing follows; add a step, or end it with `stop` or `end`"),
    ("diagram activity\nlane \"A\" {\n}\nstart\naction \"B\"\nstop", "5:1: error: this diagram uses lanes, so this `action` must be inside a `lane` block"),
    ("diagram activity\nif \"x\" {\n  action \"A\"\n}\nstop", "2:1: error: this `if` needs an incoming flow, so it cannot begin the activity; put `start` before it"),
    ("diagram activity\nstart\nacton \"Oops\"\nstop", "3:1: error: expected a statement, found `acton`; did you mean `action`?"),
    ("diagram activity\nstart\nif \"a\" {\n  stop\n} else {\n  end\n}\naction \"Dead\"", "8:1: error: this statement can never run: every branch of the `if` on line 3 ends the flow; remove it, or move it before the end"),
    ("diagram activity", "1:1: error: an activity needs at least one step, for example `start`"),
    ("diagram activity\nstart\nfork {\n  action \"A\"\n}\nstop", "3:1: error: a `fork` needs at least two branches; add `and { ... }`"),
];

#[test]
fn error_messages_keep_their_exact_text() {
    for (source, expected) in CASES {
        assert_eq!(drumlin::check(source).unwrap_err(), expected, "check: {source}");
        assert_eq!(drumlin::render(source).unwrap_err(), expected, "render: {source}");
    }
}

/// Errors earlier in a file win, whichever stage finds them.
#[test]
fn the_first_error_in_source_order_is_reported() {
    let source = "component A\n}\ncomponent @";
    assert_eq!(drumlin::check(source).unwrap_err(), "2:1: error: unmatched `}`");
}

/// A late `diagram` line is still checked, as it was before the dispatcher existed.
#[test]
fn a_late_diagram_header_is_still_rejected() {
    let source = "component A\ndiagram timing";
    assert_eq!(drumlin::check(source).unwrap_err(), "2:1: error: diagram kind `timing` is not implemented; this prototype supports `component`, `package`, `sequence`, `state`, `activity`");
}

/// A later header naming an implemented kind must not say it is unimplemented.
#[test]
fn a_late_header_of_an_implemented_kind_says_so() {
    let source = "component A\ndiagram package";
    assert_eq!(drumlin::check(source).unwrap_err(), "2:1: error: the `diagram` header must be the first statement");
}

#[test]
fn an_explicit_state_suppresses_the_typo_error() {
    assert!(drumlin::check("diagram state\nstate Idle\nstate Idel\n[*] --> Idle\nIdle --> Idel").is_ok());
}

#[test]
fn single_letter_states_are_not_typos() {
    assert!(drumlin::check("diagram state\n[*] --> A\nA --> B\nB --> [*]").is_ok());
}

#[test]
fn a_nested_package_may_depend_on_its_parent() {
    assert!(drumlin::check("diagram package\npackage P {\n  package C\n  C ..> P : <<import>>\n}").is_ok());
}

#[test]
fn a_package_may_nest_in_a_component() {
    assert!(drumlin::check("component App {\n  package Inner {\n    component X\n  }\n}").is_ok());
}

/// `layout` is a statement only when a name follows it, so an element called
/// `layout` still works in relations and hints.
#[test]
fn an_element_called_layout_is_not_the_layout_statement() {
    assert!(drumlin::check("component layout\ncomponent b\nlayout --> b\nlayout left-of b").is_ok());
    assert!(drumlin::check("diagram sequence\nparticipant layout\nparticipant b\nlayout -> b : go").is_ok());
}

/// Likewise for `style`.
#[test]
fn an_element_called_style_is_not_the_style_statement() {
    assert!(drumlin::check("component style\ncomponent b\nstyle --> b\nstyle left-of b").is_ok());
    assert!(drumlin::check("diagram sequence\nparticipant style\nparticipant b\nstyle -> b : go").is_ok());
    assert!(drumlin::check("diagram state\n[*] --> style\nstyle --> [*]").is_ok());
}

/// The default engine by name draws what no engine at all draws, and an unknown
/// name in the source is an error even when an option names an engine.
#[test]
fn the_default_engine_option_draws_like_no_option() {
    let options = drumlin::Options { layout: Some("longest-path".to_string()), ..drumlin::Options::default() };
    let source = "diagram state\n[*] --> A\nA --> B";
    assert_eq!(drumlin::render_with(source, &options), drumlin::render(source));
    let unknown = "diagram state\nlayout dot\n[*] --> A";
    assert_eq!(drumlin::render_with(unknown, &options).unwrap_err(), "2:8: error: unknown layout engine `dot`; the engines are `longest-path`, `network-simplex`");
}
