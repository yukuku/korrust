public final class ShadowCheck {
    public static void main(String[] args) {
        int value;
        if (args[0].equals("downTo")) value = next.shadow.ShadowDownToKt.shadowDownTo();
        else value = next.shadow.ShadowStepKt.shadowStep();
        int expected = args[0].equals("downTo") ? 6 : 7;
        if (value != expected) throw new AssertionError(value + " != " + expected);
        System.out.println("shadow resolved " + expected);
    }
}
