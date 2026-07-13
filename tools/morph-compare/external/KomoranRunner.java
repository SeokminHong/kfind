import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.StringJoiner;

import kr.co.shineware.nlp.komoran.constant.DEFAULT_MODEL;
import kr.co.shineware.nlp.komoran.core.Komoran;
import kr.co.shineware.nlp.komoran.model.Token;

public final class KomoranRunner {
    private KomoranRunner() {}

    public static void main(String[] args) throws Exception {
        Komoran komoran = new Komoran(DEFAULT_MODEL.FULL);
        BufferedReader input = new BufferedReader(
            new InputStreamReader(System.in, StandardCharsets.UTF_8)
        );
        Base64.Decoder decoder = Base64.getDecoder();
        Base64.Encoder encoder = Base64.getEncoder();
        String line;
        while ((line = input.readLine()) != null) {
            String text = new String(decoder.decode(line), StandardCharsets.UTF_8);
            StringJoiner tokens = new StringJoiner(";");
            for (Token token : komoran.analyze(text).getTokenList()) {
                tokens.add(
                    encoder.encodeToString(token.getMorph().getBytes(StandardCharsets.UTF_8))
                        + "," + token.getPos()
                        + "," + token.getBeginIndex()
                        + "," + token.getEndIndex()
                );
            }
            System.out.println(tokens);
        }
    }
}
