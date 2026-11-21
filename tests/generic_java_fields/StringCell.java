package genericfields;
public final class StringCell extends Cell<String> {
    public StringCell(String value) { super(value, null, java.util.Arrays.asList(value), new String[] {value}); }
}
