import boundarylib.Crate;
public class Calls {
    public static void main(String[] args) {
        if (!SupportedKt.reveal(new Crate<String>("OK")).equals("OK")) throw new AssertionError("reference extension");
        if (SupportedKt.ticket(7) != 14) throw new AssertionError("value extension");
        if (!VisibleLiteralKt.visibleName().equals("Visible")) throw new AssertionError("same-package class literal");
        System.out.println("OK");
    }
}
