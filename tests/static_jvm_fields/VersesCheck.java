import java.util.Set;
import yuku.alkitab.base.verses.ReadingGuide;
import yuku.alkitab.base.verses.VersesController.VerseSelectionMode;
import yuku.alkitab.base.verses.VersesUiModel;

public class VersesCheck {
    public static void main(String[] args) {
        VersesUiModel empty = VersesUiModel.EMPTY;
        if (empty.getTextSizeMult() != 1f || empty.getVerseSelectionMode() != VerseSelectionMode.multiple
                || empty.isVerseNumberShown() || !empty.getDictionaryModeAris().isEmpty()
                || empty.getReadingGuide() != ReadingGuide.Companion.getNONE()) throw new AssertionError();
        VersesUiModel copy = empty.copy(1.5f, VerseSelectionMode.none, true, Set.of(42), empty.getReadingGuide());
        if (copy.component1() != 1.5f || copy.component2() != VerseSelectionMode.none || !copy.component3()
                || !Set.of(42).equals(copy.component4()) || copy.component5() != empty.getReadingGuide()
                || copy.equals(empty)) throw new AssertionError();
    }
}
