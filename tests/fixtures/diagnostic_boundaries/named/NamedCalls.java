package named;
public class NamedCalls {
    public static void main(String[] args) {
        if (!AliasKt.aliased().equals("Visible")) throw new AssertionError("named alias");
        if (!PlainKt.plain().equals("Visible")) throw new AssertionError("named same-package lookup");
        System.out.println("OK");
    }
}
