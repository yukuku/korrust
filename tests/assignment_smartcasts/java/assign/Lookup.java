package assign;
public class Lookup {
    public int calls;
    @SuppressWarnings("unchecked") public <T extends Node> T find(int value) {
        calls++;
        return value == 0 ? null : (T) (value == 2 ? new Node() : new Leaf(value));
    }
    public Leaf leaf(int value) { calls++; return value == 0 ? null : new Leaf(value); }
}
