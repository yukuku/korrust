import assign.*;
public final class Calls {
    private static void check(boolean value) { if (!value) throw new AssertionError(); }
    public static void main(String[] args) {
        Lookup lookup = new Lookup();
        check(EffectsKt.literal() == 3 && EffectsKt.replace() == 5 && EffectsKt.subtype() == 7);
        lookup.calls = 0; check(EffectsKt.platform(lookup, 1) == 3 && lookup.calls == 1);
        lookup.calls = 0; check(EffectsKt.generic(lookup, 1) == 4 && lookup.calls == 1);
        check(EffectsKt.platformRead(lookup, 0) == null);
        check(EffectsKt.platformRead(lookup, 1).inputType == 1);
        lookup.calls = 0; check(EffectsKt.readonlyCapture(lookup, 1) == 5 && lookup.calls == 1);
        check(EffectsKt.beforeChangingCapture() == 3 && EffectsKt.readonlyThenAssign() == 6 && EffectsKt.shadowing() == 8);
        check(EffectsKt.branches(true) == 1 && EffectsKt.branches(false) == 4);
        check(EffectsKt.checked(null) == -1 && EffectsKt.checked("abc") == 3);
        check(EffectsKt.loop(3) == 3 && EffectsKt.readonlyLoop(2) == 9);
        try { EffectsKt.generic(lookup, 0); throw new AssertionError("null receiver accepted"); }
        catch (NullPointerException expected) { }
        try { EffectsKt.generic(lookup, 2); throw new AssertionError("wrong generic class accepted"); }
        catch (ClassCastException expected) { }
        System.out.println("OK");
    }
}
