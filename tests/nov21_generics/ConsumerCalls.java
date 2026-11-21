public class ConsumerCalls {
    static class Api implements methodgenerics.GenericApi<String> {
        public <T extends methodgenerics.Label> T identity(T value) { return value; }
        public <T> String choose(T value) { return "interface"; }
    }
    public static void main(String[] args) {
        String result = consumers.ConsumerKt.result();
        if (!result.equals("OK!seedfirstOKOK")) throw new AssertionError(result);
        if (!consumers.ConsumerKt.interfaceResult(new Api(), new methodgenerics.Named("meta")).equals("metainterface")) throw new AssertionError();
        System.out.println("OK");
    }
}
