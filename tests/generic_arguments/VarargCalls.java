import genericargs.Varargs;
import genericargs.Word;
public class VarargCalls {
    public static void main(String[] args) {
        Varargs<String> box = new Varargs<>("saved");
        String[] input = {"OK", "wrong"};
        if (box.packed(input) != input) throw new AssertionError("Java array identity");
        if (!box.first("OK", "wrong").equals("OK")) throw new AssertionError("first");
        if (box.first(7, 8) != 7) throw new AssertionError("boxing");
        if (!box.choose("wrong", "wrong", "OK").equals("OK")) throw new AssertionError("choose");
        if (!box.forward("wrong", "OK").equals("OK")) throw new AssertionError("forward");
        if (box.count("ignored", "A", "B") != 2) throw new AssertionError("count");
        if (!box.label(new Word("OK")).equals("OK")) throw new AssertionError("bound");
        if (!box.classifier(1, "OK").equals("OK")) throw new AssertionError("class");
        if (box.primitive("ignored", 7, 8) != 7) throw new AssertionError("primitive");
        if (!genericargs.VarargsKt.sourceVarargs().equals("OK")) throw new AssertionError("source calls");
        System.out.println("OK");
    }
}
