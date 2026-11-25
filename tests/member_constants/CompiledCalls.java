package compiledconstantprobe;
public class CompiledCalls {
    public static void main(String[] args) {
        if (CompiledReadKt.TOTAL != 128 || CompiledReadKt.read() != 128) throw new AssertionError();
        System.out.println("OK");
    }
}
