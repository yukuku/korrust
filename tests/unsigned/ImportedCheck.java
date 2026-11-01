public final class ImportedCheck {
    public static void main(String[] args) throws Exception {
        if (importedscalar.ImportedKt.arbitrary(2, 3) != 65) throw new AssertionError("arbitrary");
        StringBuilder log = new StringBuilder();
        if (importedscalar.ImportedKt.ordered(log) != 65 || !log.toString().equals("AB")) throw new AssertionError("order " + log);
        if (importedscalar.ImportedKt.arbitrary(Integer.MAX_VALUE, 1) != Integer.MAX_VALUE * 31 + 1) throw new AssertionError("overflow");
        if (importedscalar.ImportedKt.arrayKind().getComponentType() != Class.forName("externalvalue.Token")) throw new AssertionError("array component");
        System.out.println("OK 4");
    }
}
