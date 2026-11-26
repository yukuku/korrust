import kotlin.Unit;

public final class Calls {
    private static void check(boolean value) { if (!value) throw new AssertionError(); }
    private static void counts(int receivers, int arguments) {
        check(EffectsKt.getReceivers() == receivers);
        check(EffectsKt.getArguments() == arguments);
    }
    public static void main(String[] args) {
        Effects value = new Effects();
        Effects other = new Effects();
        EffectsKt.reset();
        check(EffectsKt.safe(null) == null); counts(1, 0);
        check(EffectsKt.safe(value) == Unit.INSTANCE); counts(2, 1);
        check(value.getCalls() == 7);
        EffectsKt.statement(null); counts(3, 1);
        EffectsKt.statement(value); counts(4, 2);
        check(value.getCalls() == 77);
        check(EffectsKt.widened(value) == Unit.INSTANCE); counts(5, 3);
        check(EffectsKt.widened(null) == null); counts(6, 3);
        EffectsKt.elvis(value, other); counts(7, 4); check(other.getCalls() == 0);
        EffectsKt.elvis(null, other); counts(8, 4); check(other.getCalls() == 3);
        EffectsKt.asserted(value); counts(9, 5);
        try { EffectsKt.asserted(null); throw new AssertionError(); }
        catch (NullPointerException expected) { counts(10, 5); }
        check(EffectsKt.nullableIdentity(Unit.INSTANCE) == Unit.INSTANCE);
        check(EffectsKt.nullableIdentity(null) == null);
        check(new UnitHolder(Unit.INSTANCE).getValue() == Unit.INSTANCE);
        check(new UnitHolder(null).getValue() == null);
        EffectsKt.force(Unit.INSTANCE);
        try { EffectsKt.force(null); throw new AssertionError(); }
        catch (NullPointerException expected) { }
        EffectsKt.reset();
        kotlin.jvm.functions.Function0<Unit> callback = EffectsKt.callback(value);
        counts(0, 0); check(callback.invoke() == Unit.INSTANCE); counts(1, 1);
        check(EffectsKt.callback(null).invoke() == null); counts(2, 1);
        check(EffectsKt.generic(value) == Unit.INSTANCE); counts(3, 2);
        check(EffectsKt.generic(null) == null); counts(4, 2);
        External external = new External();
        check(EffectsKt.external(null) == null); counts(4, 2);
        check(EffectsKt.external(external) == Unit.INSTANCE); counts(4, 3);
        check(external.calls == 7);
        external.fail = true;
        try { EffectsKt.external(external); throw new AssertionError(); }
        catch (IllegalStateException expected) { check(expected.getMessage().equals("touch")); }
        counts(4, 4);
        System.out.println("OK");
    }
}
