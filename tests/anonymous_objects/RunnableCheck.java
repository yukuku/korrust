public class RunnableCheck {
    public static void main(String[] args) {
        int[] values = new int[] { 2 };
        Runnable a = anonymousjava.RunnableKt.action(values);
        Runnable b = anonymousjava.RunnableKt.action(values);
        if (a == b || values[0] != 2) throw new AssertionError();
        a.run(); b.run();
        if (values[0] != 4) throw new AssertionError();
        System.out.println("OK");
    }
}
