public final class ImportedCheck {
    public static void main(String[] args) {
        if (importedvalue.ImportedKt.primitive() != 9) throw new AssertionError("primitive");
        if (!importedvalue.ImportedKt.reference().equals("abc")) throw new AssertionError("reference");
        if (importedvalue.ImportedKt.wide() != Long.MAX_VALUE) throw new AssertionError("wide");
        if (importedvalue.ImportedKt.nullablePrimitive() != null) throw new AssertionError("nullable primitive");
        if (importedvalue.ImportedKt.nullableReference() != null) throw new AssertionError("nullable reference");
        System.out.println("OK 5");
    }
}
