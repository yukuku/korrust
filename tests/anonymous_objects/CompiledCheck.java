public class CompiledCheck {
    public static void main(String[] args) {
        anonymouslibrary.Pair p = anonymouscompiled.CompiledKt.make(7);
        if (p.value() != 7 || p.shift(3) != 10 || !p.label().equals("pair")) throw new AssertionError();
        System.out.println("OK");
    }
}
