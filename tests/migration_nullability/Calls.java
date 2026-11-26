public final class Calls {
    private static void check(boolean value) { if (!value) throw new AssertionError(); }
    private static void fails(Runnable action) {
        try { action.run(); throw new AssertionError("missing null check"); }
        catch (NullPointerException expected) { }
    }
    public static void main(String[] args) {
        Migration.calls = 0;
        check(EffectsKt.acceptNull() == null);
        check(EffectsKt.acceptNullable(null) == null);
        check(EffectsKt.acceptNullable("input").equals("input"));
        check(EffectsKt.nullableParameter(null) == null);
        check(EffectsKt.nonNullAsNonNull(false).equals("value"));
        fails(() -> EffectsKt.nonNullAsNonNull(true));
        check(EffectsKt.nonNullAsNullable(true) == null);
        check(EffectsKt.nullableAsNonNull(false).equals("value"));
        fails(() -> EffectsKt.nullableAsNonNull(true));
        check(EffectsKt.nullableAsNullable(true) == null);
        check(EffectsKt.nullableLength(false) == 5);
        fails(() -> EffectsKt.nullableLength(true));
        check(Migration.calls == 12);
        check(EffectsKt.staticField() == null);
        EffectsKt.writeStaticField("static");
        check(EffectsKt.staticField().equals("static"));
        EffectsKt.writeStaticField(null);
        check(EffectsKt.staticField() == null);
        Migration value = new Migration();
        check(EffectsKt.readNullableField(value) == null);
        check(EffectsKt.strictFieldNonNull(value).equals("strict"));
        check(EffectsKt.strictFieldNullable(value) == null);
        fails(() -> EffectsKt.readNonNullField(value));
        EffectsKt.writeFields(value, "field");
        check(EffectsKt.readNonNullField(value).equals("field"));
        EffectsKt.writeFields(value, null);
        check(value.nullableField == null && value.nonNullField == null);
        check(EffectsKt.strictNonNull().equals("strict"));
        check(EffectsKt.strictNullable(true) == null);
        check(EffectsKt.generic(null) == null && EffectsKt.generic("generic").equals("generic"));
        System.out.println("OK");
    }
}
