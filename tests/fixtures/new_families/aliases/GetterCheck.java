package aliasverification;

import aliasboundaries.UnsupportedGenericGetterKt;
import aliasdep.Box;
import java.util.Arrays;
import java.util.List;
import java.util.function.Function;

@SuppressWarnings("unchecked")
public final class GetterCheck {
    private static int checks;

    private static void require(boolean condition) {
        checks++;
        if (!condition) throw new AssertionError("check=" + checks);
    }

    public static void main(String[] args) {
        List<Function<Box<String>, String>> readers = Arrays.asList(
            UnsupportedGenericGetterKt::read,
            UnsupportedGenericGetterKt::readWithoutAlias
        );
        for (Function<Box<String>, String> reader : readers) {
            for (String value : new String[] {"", "value", "nul\u0000", "\ud83d\ude03", null}) {
                require(reader.apply(new Box<String>(value)) == value);
            }
            try {
                reader.apply(null);
                throw new AssertionError("missing parameter null check");
            } catch (NullPointerException expected) {
                require(true);
            }
            Box<String> wrong = (Box<String>) (Box<?>) new Box<Integer>(42);
            try {
                reader.apply(wrong);
                throw new AssertionError("missing generic result cast");
            } catch (ClassCastException expected) {
                require(true);
            }
        }
        System.out.println("checks=" + checks);
    }
}
