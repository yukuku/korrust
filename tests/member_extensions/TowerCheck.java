public final class TowerCheck {
    public static void main(String[] args) {
        Value value = new Value(1);
        Mutation mutation = new Mutation();
        if (mutation.result(value) != 101 || mutation.getAmount() != 99 || value.getAmount() != 2) throw new AssertionError("implicit mutation priority");
        if (!new Tower("outer").nested(new Tower("inner")).equals("Oinner")) throw new AssertionError("nearest dispatch receiver");
        Value custom = new Value(1);
        if (new CustomMutation().result(custom) != 101 || custom.getAmount() != 2) throw new AssertionError("custom dispatch accessor priority");
        StaticScope scope = new VirtualScope();
        DerivedValue derived = new DerivedValue();
        if (!scope.base(derived).equals("child") || !scope.derived(derived).equals("derived")) throw new AssertionError("static extension resolution and virtual dispatch");
        if (!new Priority().result().equals("member")) throw new AssertionError("real member priority");
        if (!new Generic<String>().result("OK").equals("OK") || !new GenericProperty<String>().result("OK").equals("OK")) throw new AssertionError("class generic receiver");
        if (!new Effects().result().equals("RAAAA")) throw new AssertionError("receiver, arguments, and defaults order");
        if (!new MethodGeneric().result().equals("OK")) throw new AssertionError("method generic receiver");
        if (!new ChildGeneric().run("O").equals("OK")) throw new AssertionError("generic member extension bridge");
        if (TowerKt.labeled() != 3 || new Props().result() != 2) throw new AssertionError("receiver labels");
    }
}
