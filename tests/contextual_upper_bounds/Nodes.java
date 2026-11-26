public class Nodes {
    public static class Node {}
    public static class Leaf extends Node {}
    public static final Leaf first = new Leaf();
    @SuppressWarnings("unchecked")
    public static <T extends Node> T find(int id) { return (T)(id == 0 ? null : first); }
    public static boolean same(Object left, Object right) { return left == right; }
}
