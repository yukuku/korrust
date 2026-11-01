public class AliasCalls {
    public static void main(String[] args) {
        if (!AliasLiteralKt.aliasName().equals("Visible")) throw new AssertionError("alias priority");
        if (!AliasHiddenKt.hiddenAlias().equals("Visible")) throw new AssertionError("same-package original");
        System.out.println("OK");
    }
}
