package org.acme;

import jakarta.persistence.Cacheable;
import jakarta.persistence.Column;
import jakarta.persistence.Entity;
import jakarta.persistence.GeneratedValue;
import jakarta.persistence.Id;

@Entity
@Cacheable
public class Basket {
    @Id
    @GeneratedValue
    public Long id;

    @Column(length = 40, unique = true)
    public String label;
}
