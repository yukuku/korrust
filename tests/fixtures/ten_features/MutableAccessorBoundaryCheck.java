public final class MutableAccessorBoundaryCheck {
    private static int checks;
    private static void check(boolean value, String message) { checks++; if (!value) throw new AssertionError(message); }
    public static void main(String[] args) {
        E first = new E(), second = new E();
        check(first.getValue() == 1 && second.getValue() == 1, "instance initializers");
        first.setValue(9);
        check(first.getValue() == 9 && second.getValue() == 1, "setter backing field and instance isolation");
        first.setValue(Integer.MIN_VALUE);
        check(first.getValue() == Integer.MIN_VALUE, "minimum setter value");
        first.setValue(Integer.MAX_VALUE);
        check(first.getValue() == Integer.MAX_VALUE, "maximum setter value");
        System.out.println("{\"id\":\"custom_mutable_accessor\",\"checks\":" + checks + "}");
    }
}
