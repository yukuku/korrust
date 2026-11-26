public class Caller {
    public static void main(String[] args) {
        try { ConsumerKt.text(); throw new AssertionError(); }
        catch (IllegalArgumentException e) { if (!"text".equals(e.getMessage())) throw e; }
        try { ConsumerKt.number(); throw new AssertionError(); }
        catch (IllegalArgumentException e) { if (!"number".equals(e.getMessage())) throw e; }
        try { ConsumerKt.standard(); throw new AssertionError(); }
        catch (IllegalStateException e) { if (!"standard".equals(e.getMessage())) throw e; }
        if (ConsumerKt.optional() != null) throw new AssertionError();
    }
}
