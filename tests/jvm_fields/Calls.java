package fieldapi;
import java.lang.reflect.Modifier;
import java.util.Objects;
public final class Calls {
    private static void eq(Object expected, Object actual) {
        if (!Objects.equals(expected, actual)) throw new AssertionError("expected=" + expected + " actual=" + actual);
    }
    private static void field(Class<?> owner, String name, Class<?> type, int flags) throws Exception {
        java.lang.reflect.Field value = owner.getDeclaredField(name);
        eq(type, value.getType());
        eq(flags, value.getModifiers());
    }
    public static final class Child extends Protected {
        String read() { return value; }
    }
    public static void main(String[] args) throws Exception {
        Capsule value = new Capsule("name", 2, "normal");
        eq("name", value.name);
        eq(2, value.count);
        eq("name!", value.derived);
        eq(null, value.absent);
        eq(null, value.maybe);
        eq(true, value.isReady);
        eq(7L, value.ticks);
        value.count = 8;
        value.label = "java";
        value.ticks = 11L;
        eq(9, value.increment());
        eq("java", value.label);
        eq(11L, value.ticks);
        eq("normal", value.getOrdinary());
        eq("aabic:aabc", ModelsKt.ordered());
        eq("name4changednormal", ModelsKt.ordinary());
        eq("name", new GenericBox<String>("name").item);
        eq("direct", new AccessorName("direct").value);
        eq("explicit", new AccessorName("direct").getValue());
        eq("protected", new Child().read());
        field(Capsule.class, "name", String.class, Modifier.PUBLIC | Modifier.FINAL);
        field(Capsule.class, "count", int.class, Modifier.PUBLIC);
        field(Capsule.class, "ticks", long.class, Modifier.PUBLIC);
        field(Capsule.class, "ordinary", String.class, Modifier.PRIVATE | Modifier.FINAL);
        for (String method : new String[]{"getName", "getCount", "getDerived", "getLabel", "getAbsent", "getMaybe", "isReady", "getTicks"}) {
            for (java.lang.reflect.Method declared : Capsule.class.getDeclaredMethods()) {
                if (declared.getName().equals(method)) throw new AssertionError("unexpected accessor " + method);
            }
        }
        Legacy legacy = new Legacy();
        try { legacy.getText(); throw new AssertionError(); }
        catch (kotlin.UninitializedPropertyAccessException expected) {}
        legacy.setText("legacy");
        eq("legacy", legacy.getText());
        Shadow shadow = new Shadow("shadow");
        eq("shadow", shadow.getValue());
        field(Shadow.class, "value", String.class, Modifier.PRIVATE | Modifier.FINAL);
        eq("local", new fieldmarker.LocalShadow("local").getValue());
        field(fieldmarker.LocalShadow.class, "value", String.class, Modifier.PRIVATE | Modifier.FINAL);
        eq("inferred", ModelsKt.guessed());
        eq("inferred", new Guessed().text);
        System.out.println("OK");
    }
}
