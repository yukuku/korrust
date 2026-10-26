public final class InitBlockBoundaryCheck {
    public static void main(String[] args) throws Exception {
        E first = new E();
        E second = new E();
        if (first.getClass() != E.class) throw new AssertionError("class");
        if (first == second) throw new AssertionError("fresh construction");
        if (!java.lang.reflect.Modifier.isPublic(E.class.getDeclaredConstructor().getModifiers())) throw new AssertionError("constructor");
        System.out.println("{\"id\":\"init_block\",\"checks\":3}");
    }
}
