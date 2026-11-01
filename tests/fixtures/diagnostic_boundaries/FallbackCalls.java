public class FallbackCalls {
    public static void main(String[] args) throws Exception {
        if (!FallbackKt.fallbackProbe().equals("packet")) throw new AssertionError("wrong overload");
        System.out.println("OK");
    }
}
