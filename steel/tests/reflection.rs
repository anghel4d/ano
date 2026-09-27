mod support;
use support::Fixture;

const WORLD: &str = "n 3\ncol All bool 1 1 1\ncol Gold num 1 2 3\ncol Path num 2 0 1\ncol Marked bool 0 0 0\ncol Race sym Nord Breton Nord\nrel parent -1 0 1\ninv children parent\nas Money Gold\nja 金 Gold\nfn show\nfn legacy {𝕩}\nfn pair {𝕨+𝕩}\nfn add id:0000000000000001 v:1 sig:num,num->num fx:pure det:deterministic trust:trusted read:- write:- use:- = {𝕨+𝕩}\nfn insert id:0000000000000002 v:1 sig:unit->unit fx:write det:deterministic trust:trusted read:- write:Gold,Marked use:- = {𝕩}\nfn cancel id:0000000000000003 v:1 sig:unit->unit fx:write det:deterministic trust:trusted read:- write:Gold use:- = {𝕩}\n";

#[test]
fn category_casts_check_identity_without_reading_or_invoking() {
    let f = Fixture::new(WORLD);
    f.accepts(
        r#"col_"Gold" == col_"Money"
--! out 1
col_"gold" == col_"Gold"
--! out 1
col_"Gold" != col_"Marked"
--! out 1
col_"Gold".name
--! out Gold
col_"金".type
--! out num
col_"Marked".type
--! out bool
col_"Race".type
--! out sym
col_"parent".type
--! out entity
col_"children".type
--! out entities
col_"Gold".domain
--! out entity
fun_"insert".name
--! out insert
--! expect Gold = 1 2 3
--! expect Marked = 0 0 0
"#,
    );
    for source in [r#"fun_"Gold""#, r#"col_"add""#] {
        f.refuses(source, "not a");
    }
    f.refuses(r#"col_"Missing""#, "unknown column declaration");
    f.refuses(r#"fun_"Missing""#, "unknown function declaration");
}

#[test]
fn reflection_footprints_compose_with_existing_generators_and_guards() {
    let f = Fixture::new(WORLD);
    f.accepts(
        r#"[fun_"insert".writes -> c |=> c.name]
--! out Gold Marked
[fun_"insert".writes -> a & fun_"cancel".writes -> b & a == b |=> a.name]
--! out Gold
[[col_"Gold", col_"Race", col_"Marked"] -> c & c.type == :num |=> c.name]
--! out Gold
[fun_"add".inputs -> t & t == :num |=> 1]
--! out 1 1
fun_"add".result
--! out num
fun_"add".arity + 1
--! out 3
fun_"add".writes
--! out
--! expect Gold = 1 2 3
"#,
    );
    f.accepts(
        r#"def writes = fun_"insert".writes
[writes -> c |=> c.name]
--! out Gold Marked
"#,
    );
}

#[test]
fn explicit_callable_references_feed_existing_reducer_contracts() {
    let f = Fixture::new(WORLD);
    f.accepts(
        r#"fold(fun_"add", Gold)
--! out 6
scan(fun_"add", Gold)
--! out 1 3 6
def reducer = fun_"add"
scan(reducer, Gold)
--! out 1 3 6
fold(fun_"max", Gold)
--! out 3
scan(fun_"add", Gold, Path)
--! out 3 4 6
cross(fun_"pair", All, All)
--! out 0 1 2 1 2 3 2 3 4
"#,
    );
    f.refuses(r#"scan(col_"Gold", Gold)"#, "requires a fun_");
    f.refuses(r#"scan(fun_"insert", Gold)"#, "reducer");
    f.refuses(
        r#"show(col_"Gold")"#,
        "show arguments must name entity columns",
    );
    f.accepts("All , show(Gold)\n--! expect Gold = 1 2 3");
}

#[test]
fn reflection_refuses_unavailable_metadata_and_world_scatter() {
    let f = Fixture::new(WORLD);
    f.refuses(r#"col_"Gold".writes"#, "unavailable");
    f.refuses(r#"fun_"legacy".reads"#, "no declared");
    f.refuses(r#"fun_"legacy".inputs"#, "no declared");
    f.refuses(r#"col_"Gold".missing"#, "unknown reflection property");
    f.refuses(r#"All , Gold = col_"Gold""#, "cannot write value result");
    f.refuses(r#"selection([col_"Gold"]) , +Marked"#, "entity references");
    f.refuses(r#"col_"""#, "needs a declaration name");
    f.refuses(r#"fun_"add"()"#, "unexpected token");
    f.accepts(
        r#"--! ja
col_"金" . name
--! out Gold
"#,
    );
}

#[test]
fn nominal_array_columns_are_reflected_without_reading_a_payload() {
    let f = Fixture::new("n 0\nenum Faction id:0000000000000001 v:1 Nord=1 Breton=2 reserve:-\narray Race id:0000000000000002 v:1 Faction entity\n");
    f.accepts(
        r#"col_"Race".type
--! out Faction
col_"Race".domain
--! out entity
"#,
    );
    f.refuses(r#"col_"Faction""#, "not a column");
}

#[test]
fn reflected_constraints_describe_enforced_publication_and_laws() {
    let f = Fixture::new(&format!("{WORLD}range Gold 0 10\n"));
    f.accepts(
        r#"col_"Gold".range
--! out 0 10
[col_"Gold".constraints -> c & c == :nonnegative |=> 1]
--! out 1
[col_"Marked".constraints -> c & c == :integer |=> 1]
--! out 1
All , Gold = 20
--! expect Gold = 10 10 10
fun_"insert".arity
--! out 0
[fun_"insert".effects -> e & e == :write |=> 1]
--! out 1
fun_"add".determinism
--! out deterministic
fun_"add".trust
--! out trusted
fun_"add".associative
--! out undeclared
fun_"+".associative
--! out fails
fun_"+".commutative
--! out holds
fun_"max".associative
--! out holds
fun_"add".monotonic
--! out undeclared
(10000000000000000 + -10000000000000000) + 1
--! out 1
10000000000000000 + (-10000000000000000 + 1)
--! out 0
"#,
    );
}
