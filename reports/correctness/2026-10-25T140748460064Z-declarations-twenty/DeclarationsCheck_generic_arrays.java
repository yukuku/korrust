import declarations.probes.*;
import declarations.library.*;
import declarations.support.Trace;
import java.util.Arrays;
public final class DeclarationsCheck_generic_arrays {
    static int checks;
    static void check(boolean condition) { checks++; if (!condition) throw new AssertionError("check " + checks + " events=" + Trace.events); }
    static void events(String expected) { check(Trace.events.equals(expected)); }
    public static void main(String[] args) {
        Trace.clear();
        switch (args[0]) {
            case "generic_arrays": check(Arrays.equals(ArraysKt.makeArray(),new String[]{"v0","v1","v2"})); events("012"); check(ArraysKt.arrayTotal(new String[]{"ab",null,"c"}) == 3); check(ArraysKt.arrayTotal(new String[]{}) == 0); check(ArraysKt.integerArray() == 11); ArrayItem[] items = ArraysKt.objectArray(); check(items.getClass().getComponentType() == ArrayItem.class); check(items[0].getValue() == 4 && items[1].getValue() == 5); break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"checks\":" + checks + "}");
    }
}
