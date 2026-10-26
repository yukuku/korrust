import compatibility.declarations.*;
import java.lang.reflect.Method;
import java.util.Objects;
import kotlin.jvm.functions.Function1;
import kotlin.Unit;

public final class DeclarationsCheck_type_aliases {
    static int checks;
    static void eq(Object actual, Object expected) {
        checks++;
        if (!Objects.equals(actual, expected)) throw new AssertionError("expected " + expected + ", got " + actual);
    }
    public static void main(String[] args) throws Exception {
        switch (args[0]) {
            case "type_aliases":
                eq(AliasesKt.label("value"), "value");
                eq(AliasesKt.privateAliasBody("hidden"), "hidden");
                eq(AliasesKt.aliasApply(5, value -> value + 10), 15);
                eq(AliasUsesKt.chainedAlias("chain"), "chain");
                eq(AliasUsesKt.callbackAlias(4), 9);
                break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"id\":\"" + args[0] + "\",\"checks\":" + checks + "}");
    }
}
