import java.lang.reflect.Field;
import java.lang.reflect.Modifier;

public final class Calls {
    private static void field(Class<?> owner, String name, Class<?> type, boolean shared) throws Exception {
        Field value = owner.getDeclaredField(name);
        if (value.getType() != type || !Modifier.isVolatile(value.getModifiers()) || Modifier.isStatic(value.getModifiers()) != shared || Modifier.isFinal(value.getModifiers())) throw new AssertionError(value);
        if (value.getDeclaredAnnotations().length != 0) throw new AssertionError("SOURCE marker retained");
    }
    public static void main(String[] args) throws Exception {
        field(Storage.class, "instance", Storage.class, true);
        field(Storage.class, "published", long.class, true);
        field(Storage.class, "count", int.class, false);
        field(Storage.class, "text", String.class, false);
        field(State.class, "flag", boolean.class, true);
        field(FieldsKt.class, "top", long.class, true);
        field(Counter.class, "value", int.class, false);
        if (Storage.Companion.getClass().getDeclaredFields().length != 0) throw new AssertionError("wrong field owner");
        Storage first = new Storage();
        Storage second = new Storage();
        if (Storage.exchange(first) != null || Storage.exchange(second) != first || Storage.exchange(null) != second) throw new AssertionError("cached receiver");
        Storage.Companion.setPublished(Long.MAX_VALUE);
        if (Storage.Companion.getPublished() != Long.MAX_VALUE) throw new AssertionError("long");
        first.setCount(42); first.setText("value");
        if (first.getCount() != 42 || !first.getText().equals("value")) throw new AssertionError("instance");
        Counter counter = new Counter(6); counter.setValue(9);
        if (counter.getValue() != 9) throw new AssertionError("constructor");
        State.INSTANCE.setFlag(true); FieldsKt.setTop(Long.MIN_VALUE);
        if (!State.INSTANCE.getFlag() || FieldsKt.getTop() != Long.MIN_VALUE) throw new AssertionError("state");
        System.out.println("OK");
    }
}
