package priority.consumer;
import java.lang.reflect.Modifier;
public final class Calls {
    private static void eq(Object expected, Object actual) {
        if (!java.util.Objects.equals(expected, actual)) throw new AssertionError("expected=" + expected + " actual=" + actual);
    }
    public static void main(String[] args) throws Exception {
        priority.one.StringBuilder one = new priority.one.StringBuilder(11);
        priority.two.StringBuilder two = new priority.two.StringBuilder(22);
        Wildcard wildcard = new Wildcard("wildcard", one, one);
        eq("wildcard", wildcard.getValue());
        eq(one, wildcard.getBuilder());
        eq(one, wildcard.getAlias());
        eq("wildcard", WildcardKt.read(wildcard));
        eq(Modifier.PRIVATE | Modifier.FINAL, Wildcard.class.getDeclaredField("value").getModifiers());
        eq(priority.one.StringBuilder.class, Wildcard.class.getDeclaredField("builder").getType());
        Explicit explicit = new Explicit("explicit", two);
        eq("explicit", explicit.value);
        eq(two, explicit.getBuilder());
        eq(Modifier.PUBLIC | Modifier.FINAL, Explicit.class.getDeclaredField("value").getModifiers());
        priority.local.StringBuilder local = new priority.local.StringBuilder(33);
        priority.local.SamePackage same = new priority.local.SamePackage("local", local);
        eq("local", same.getValue());
        eq(local, same.getBuilder());
        Aliased aliased = new Aliased("alias", one);
        eq("alias", aliased.value);
        eq(one, aliased.getBuilder());
        eq(one, new Duplicate(one).getBuilder());
        System.out.println("OK");
    }
}
