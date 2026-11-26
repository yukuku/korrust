package aliases.consumer;
import java.util.Map;
public class Calls {
    public static void main(String[] args) {
        if (!ConstructorsKt.local().getValue().equals("local")) throw new AssertionError();
        if (!ConstructorsKt.privateLocal().equals("private")) throw new AssertionError();
        if (ConstructorsKt.swap().first != 8 || !ConstructorsKt.swap().second.equals("swap")) throw new AssertionError();
        if (!ConstructorsKt.fixed().first.equals("fixed") || ConstructorsKt.fixed().second != 9) throw new AssertionError();
        if (!ConstructorsKt.known().first.isEmpty() || !ConstructorsKt.known().second.equals("known")) throw new AssertionError();
        if (!ConstructorsKt.repeated().first.equals("one") || !ConstructorsKt.repeated().second.equals("two")) throw new AssertionError();
        if (!ConstructorsKt.nested().value.get(0).equals("nested")) throw new AssertionError();
        if (ConstructorsKt.defaults().getValue() != null || ConstructorsKt.nullDefault().getValue() != null) throw new AssertionError();
        if (ConstructorsKt.callback().getCallback().invoke("word") != 4) throw new AssertionError();
        if (!ConstructorsKt.imported().value.equals("imported") || !ConstructorsKt.chained().value.equals("chained")) throw new AssertionError();
        if (!ConstructorsKt.bound().value.equals("bound")) throw new AssertionError();
        if (!ConstructorsKt.nullable().value.equals("nullable") || !ConstructorsKt.generic("generic").getValue().equals("generic")) throw new AssertionError();
        if (!ConstructorsKt.callableShadow(value -> "shadow:" + value).equals("shadow:callable")) throw new AssertionError();
        if (!ConstructorsKt.standard(Map.of("key", "copy")).get("key").equals("copy")) throw new AssertionError();
        if (!ConstructorsKt.evaluation().equals("BA:ABK")) throw new AssertionError();
        System.out.println("typealias-constructor-ok");
    }
}
