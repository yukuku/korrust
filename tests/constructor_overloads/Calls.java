package constructors;
import java.lang.reflect.*;
public final class Calls {
    static void check(boolean value, String message) { if (!value) throw new AssertionError(message + ": " + Effects.log); }
    public static void main(String[] args) throws Exception {
        Effects.reset();
        Interleaved first = new Interleaved("R", 8);
        check(first.getA() == 1 && first.getB() == 2L && first.getC() == 3 && first.getTail() == 8 && Effects.log.equals("1234"), "all omitted");
        Effects.reset();
        Interleaved second = new Interleaved(11, "R", 8);
        check(second.getA() == 11 && second.getB() == 2L && second.getC() == 3 && Effects.log.equals("234"), "interleaved omitted");
        Effects.reset();
        Interleaved third = new Interleaved(11, "R", 12L, 8);
        check(third.getB() == 12L && third.getC() == 3 && Effects.log.equals("34"), "one omitted");
        Effects.reset();
        Dependent dep = new Dependent();
        check(dep.getA() == 5 && dep.getB() == 11 && Effects.log.equals("56"), "dependent order");
        Effects.reset();
        try { new Exceptional("R"); throw new AssertionError("missing exception"); }
        catch (IllegalStateException e) { check(e.getMessage().equals("default failed") && Effects.log.equals("7X"), "throw stops defaults"); }
        Effects.reset();
        try { new Exceptional(null); throw new AssertionError("missing null exception"); }
        catch (NullPointerException e) { check(Effects.log.equals(""), "null check before defaults"); }
        Effects.reset();
        Child child = new Child();
        check(child.getValue() == 9 && Effects.log.equals("9"), "super constructor");
        check(new Record("R").getCount() == 2 && new Internal().getValue() == 10, "data and internal classes");
        Effects.reset();
        check(new Mask32().getP31() == 1 && Effects.log.equals("1"), "signed mask");
        Constructor<Decorated> reduced = Decorated.class.getConstructor(String.class);
        check(reduced.getAnnotation(Mark.class).value().equals("constructor"), "constructor annotation");
        check(((Mark)reduced.getParameterAnnotations()[0][0]).value().equals("text"), "parameter annotation remap");
        check(custom.Lookalike.class.getConstructors().length == 2, "lookalike never lowered");
        check(NoDefaults.class.getConstructors().length == 1, "no defaults");
        Wide wide = new Wide(7L, "tail");
        check(wide.getRequired() == 7L && wide.getFirst() == 1.5 && wide.getSecond() == 2.5f && wide.getTail().equals("tail"), "wide retained slots");
        check(new Nullable(1).getText() == null, "nullable default");
        check(new Automatic().getValue() == 13 && Automatic.class.getConstructor().getAnnotation(Mark.class).value().equals("auto"), "automatic noarg annotation");
        System.out.println("OK");
    }
}
