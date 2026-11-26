public final class Calls {
    private static void check(boolean value) { if (!value) throw new AssertionError(); }
    public static void main(String[] args) {
        check(EffectsKt.mutate() == 2);
        check(EffectsKt.direct());
        check(EffectsKt.mutableView() == 1 && EffectsKt.readonlyView() == 0);
        check(EffectsKt.readonlyArgument(java.util.Arrays.asList("a")) == 1);
        check(EffectsKt.mutableArgument(new java.util.ArrayList<CharSequence>()) == 0);
        check(EffectsKt.explicitReadonlyArgument(java.util.Arrays.asList("a")) == 1);
        check(EffectsKt.explicitMutableArgument(new java.util.ArrayList<CharSequence>()) == 0);
        check(EffectsKt.nullable() == 1 && EffectsKt.platform() == 1);
        System.out.println("OK");
    }
}
