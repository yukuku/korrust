public final class Calls {
    public static void main(String[] args) {
        java.util.List<String> value = java.util.Arrays.asList("a");
        if (EffectsKt.readonlyOut(value) != 1 || EffectsKt.readonlyView(new java.util.ArrayList<String>()) != 0 || !EffectsKt.identityOut(value).get(0).equals("a") || new Holder().accept(value) != 1 || EffectsKt.alias(value) != 1) throw new AssertionError();
        System.out.println("OK");
    }
}
