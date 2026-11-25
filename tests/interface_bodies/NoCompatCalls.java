public class NoCompatCalls {
    public static void main(String[] args) {
        if (!nocompatconsumer.NoCompatClassKt.value().equals("ok")) throw new AssertionError();
        if (!new nocompatconsumer.Direct().echo("java").equals("java")) throw new AssertionError();
        System.out.println("OK");
    }
}
