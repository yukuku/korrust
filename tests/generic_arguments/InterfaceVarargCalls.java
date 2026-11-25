import genericargs.InterfaceVarargApi;
import genericargs.InterfaceVarargs;
import genericargs.InterfaceLabel;
import genericargs.InterfaceWord;
public class InterfaceVarargCalls {
    static class JavaApi implements InterfaceVarargApi<String> {
        public <R> int count(R... items) { return items.length; }
        public <R extends InterfaceLabel> String label(R... items) { return items[0].text(); }
        public <R> String select(R ignored, String... items) { return items[0]; }
    }
    public static void main(String[] args) {
        InterfaceVarargApi<String> api = new InterfaceVarargs<>();
        if (api.count(1,2) != 2) throw new AssertionError("count");
        if (!api.label(new InterfaceWord("OK")).equals("OK")) throw new AssertionError("bound");
        if (!api.select(1, "OK").equals("OK")) throw new AssertionError("classifier");
        if (!genericargs.InterfaceVarargsKt.sourceInterfaceVarargs().equals("OK")) throw new AssertionError("source");
        if (!genericargs.InterfaceVarargsKt.throughInterface(new JavaApi()).equals("OK")) throw new AssertionError("Java implementation");
        System.out.println("OK");
    }
}
