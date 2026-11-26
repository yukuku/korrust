public class Caller {
    public static void main(String[] args) throws Exception {
        Class<?> c = Class.forName("constants.ConsumerKt");
        if (c.getField("COLOR").getInt(null) != 0xfffff59d || c.getField("LOW").getInt(null) != -1) throw new AssertionError();
        if (c.getField("HUGE").getLong(null) != Long.MAX_VALUE || c.getField("SMALL").getInt(null) != Integer.MIN_VALUE) throw new AssertionError();
        if (c.getField("TRUNCATED").getInt(null) != -3 || c.getField("ZERO").getInt(null) != 0) throw new AssertionError();
        if (c.getField("WIDENED").getLong(null) != -1L) throw new AssertionError();
        if (c.getField("ROUNDED").getFloat(null) != 16777216f || c.getField("DOUBLE").getDouble(null) != 16777216d) throw new AssertionError();
    }
}
