class PendingCheck {
    public static void main(String[] args) {
        if (PendingKt.pending(null) != 0 || PendingKt.pending(3) != 12) {
            throw new AssertionError();
        }
        if (PendingKt.wide(new String[] {"abc", null, "d"}) != 11L) {
            throw new AssertionError();
        }
    }
}
