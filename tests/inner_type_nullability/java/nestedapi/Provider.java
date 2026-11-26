package nestedapi;
import org.jspecify.annotations.*;

public final class Provider {
    public Outer.@Nullable Leaf field;
    public static Outer.Middle.@Nullable Leaf staticField;
    public static Outer.@Nullable Leaf nullable(boolean missing) { return missing ? null : new Outer.Leaf(4); }
    public static Outer.@NonNull Leaf nonNull() { return new Outer.Leaf(6); }
    public static Outer.Middle.@Nullable Leaf nested(boolean missing) { return missing ? null : new Outer.Middle.Leaf(8); }
    public static int accept(Outer.@Nullable Leaf value) { return value == null ? -1 : value.value; }
    public static int strict(Outer.@NonNull Leaf value) { return value.value; }
    public static Outer.@Nullable Leaf[] arrayElement() { return new Outer.Leaf[] { null }; }
    public static java.util.List<Outer.@Nullable Leaf> genericElement() { return null; }
}
