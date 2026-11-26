package nestedapi;
public final class Outer {
    public static final class Leaf {
        public int value;
        public Leaf(int value) { this.value = value; }
    }
    public static final class Middle {
        public static final class Leaf {
            public int value;
            public Leaf(int value) { this.value = value; }
        }
    }
}
