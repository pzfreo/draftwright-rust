/* Hand-built PMI practice §7.4 part notes, in Specify's AttributeSet form.
   Inject before ENDSEC in parts/plate.step: #4 is its product_definition_shape,
   #713 its representation context. No presentation, producer metadata or callouts. */
#50000=DESCRIPTIVE_REPRESENTATION_ITEM('general note','ETCH PART NUMBER');
#50001=GENERAL_PROPERTY('','semantic text',$);
#50002=PROPERTY_DEFINITION('semantic text','user defined attribute',#4);
#50003=REPRESENTATION('',(#50000),#713);
#50004=PROPERTY_DEFINITION_REPRESENTATION(#50002,#50003);
#50005=GENERAL_PROPERTY_ASSOCIATION('',$,#50001,#50002);
#50010=DESCRIPTIVE_REPRESENTATION_ITEM('surface treatment','ANODIZE BLACK');
#50011=GENERAL_PROPERTY('','semantic text',$);
#50012=PROPERTY_DEFINITION('semantic text','user defined attribute',#4);
#50013=REPRESENTATION('',(#50010),#713);
#50014=PROPERTY_DEFINITION_REPRESENTATION(#50012,#50013);
#50015=GENERAL_PROPERTY_ASSOCIATION('',$,#50011,#50012);
#50020=DESCRIPTIVE_REPRESENTATION_ITEM('heat treatment','HARDEN TO 40 HRC');
#50021=GENERAL_PROPERTY('','semantic text',$);
#50022=PROPERTY_DEFINITION('semantic text','user defined attribute',#4);
#50023=REPRESENTATION('',(#50020),#713);
#50024=PROPERTY_DEFINITION_REPRESENTATION(#50022,#50023);
#50025=GENERAL_PROPERTY_ASSOCIATION('',$,#50021,#50022);
