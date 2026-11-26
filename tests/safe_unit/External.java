public final class External {
    public int calls;
    public boolean fail;
    public void touch(int value) {
        if (fail) throw new IllegalStateException("touch");
        calls = calls * 10 + value;
    }
}
