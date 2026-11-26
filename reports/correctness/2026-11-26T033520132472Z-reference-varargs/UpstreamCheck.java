public final class UpstreamCheck { public static void main(String[] args) { if (!RawStringsWithManyQuotesKt.box().equals("OK")) throw new AssertionError(); System.out.println("OK"); } }
