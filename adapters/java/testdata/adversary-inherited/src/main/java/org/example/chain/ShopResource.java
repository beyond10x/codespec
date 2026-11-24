package org.example.chain;

public class ShopResource implements ShopApi {
    @Override
    public String ping() {
        return "pong";
    }
}
