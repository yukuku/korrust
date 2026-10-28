package newfamilies.aliases;

import aliasdep.Box;
import aliasdep.Bounded;
import java.util.Arrays;
import java.util.Collections;
import kotlin.jvm.functions.Function1;
import kotlin.jvm.functions.Function2;

public final class AliasesCheck {
    private static int checks;
    private static int calls;

    private static void require(boolean condition) {
        checks++;
        if (!condition) throw new AssertionError("check=" + checks);
    }

    public static void main(String[] args) {
        require(UsesKt.count(Arrays.asList("a", "b")) == 2);
        require(UsesKt.count(Collections.emptyList()) == 0);
        require(UsesKt.nullableCount(null) == -1);
        require(UsesKt.nullableCount(Arrays.asList("a", "b", "c")) == 3);
        require(UsesKt.sum(Arrays.asList(Integer.MIN_VALUE, -1)) == Integer.MAX_VALUE);
        require(UsesKt.sum(Arrays.asList(19, 23)) == 42);
        require(UsesKt.firstNested(Collections.singletonList(Collections.singletonList(null))) == null);
        require("value".equals(UsesKt.firstNested(Collections.singletonList(Collections.singletonList("value")))));
        require("ignored".equals(UsesKt.ignored("ignored")));
        Function1<String, String> fallback = value -> {
            calls++;
            return value == null ? "fallback" : value;
        };
        require("fallback".equals(UsesKt.apply(null, fallback)));
        require(calls == 1);
        require("present".equals(UsesKt.apply("present", fallback)));
        require(calls == 2);
        String[] slots = new String[] {null, "tail"};
        require(UsesKt.swap(slots) == null);
        require("changed".equals(slots[0]));
        require("tail".equals(slots[1]));
        require("changed".equals(UsesKt.swap(slots)));
        require("n=42".equals(UsesKt.convert(42, value -> "n=" + value)));
        Function2<String, Integer, String> formatter = (value, number) -> value + number;
        require("R3".equals(UsesKt.format("R", formatter)));
        require("none".equals(UsesKt.optional(null)));
        require("OK!".equals(UsesKt.optional(value -> value + "!")));
        Holder holder = new Holder(fallback, Arrays.asList("x"));
        require("fallback".equals(holder.getTransform().invoke(null)));
        require(calls == 3);
        require(holder.getRows().size() == 1);
        require(DefinitionsKt.privateAliasSize(Arrays.asList(1, 2)) == 2);
        Box<String> box = new Box<String>("box");
        require(UsesKt.boxed(box) == box);
        require("box".equals(UsesKt.boxed(box).getValue()));
        Bounded<String> bounded = new Bounded<String>("bound");
        require(UsesKt.constrained(bounded) == bounded);
        require("bound".equals(UsesKt.constrained(bounded).getValue()));
        IllegalStateException failure = new IllegalStateException("identity");
        try {
            UsesKt.apply("x", value -> { throw failure; });
            throw new AssertionError("missing exception");
        } catch (IllegalStateException actual) {
            require(actual == failure);
        }
        try {
            UsesKt.firstNested(Collections.emptyList());
            throw new AssertionError("missing bounds check");
        } catch (IndexOutOfBoundsException expected) {
            require(true);
        }
        try {
            UsesKt.ignored(null);
            throw new AssertionError("missing nonnull parameter check");
        } catch (NullPointerException expected) {
            require(true);
        }
        require(UsesKt.countTriple(Collections.singletonList(Collections.singletonList(Collections.singletonList("triple")))) == 1);
        require(UsesKt.countMutableNested(Collections.singletonList(Arrays.asList("mutable"))) == 1);
        require(UsesKt.countMutableOuter(Collections.singletonList(Collections.singletonList("outer"))) == 1);
        require(UsesKt.countNullableNested(Collections.singletonList(null)) == 1);
        Function1<Integer, String> numberFunction = value -> "n=" + value;
        require(UsesKt.countFunctions(Collections.singletonList(numberFunction)) == 1);
        require(UsesKt.countBoxRows(Collections.singletonList(box)) == 1);
        require(UsesKt.countArrayRows(Collections.singletonList(new String[] {"array"})) == 1);
        require("generic".equals(UsesKt.genericFirst(Collections.singletonList("generic"))));
        require("fallback".equals(UsesKt.genericApply(null, fallback)));
        System.out.println("checks=" + checks);
    }
}
