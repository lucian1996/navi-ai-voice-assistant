"use client";
import React, { useEffect, useState } from "react";
import ChatFooter from "@/components/chat/chat-footer";
import ChatMain from "@/components/chat/chat-main";
import SideBar from "@/components/sidebar/sidebar";
import Header from "@/components/header/header";
import useNexus from "@/store";

export default function Home() {
  const [isMobile, setIsMobile] = useState(false);
  const setMobile = useNexus((state) => state.setMobile);

  useEffect(() => {
    const checkIsMobile = () => {
      setIsMobile(window.innerWidth < 350);
    };
    checkIsMobile();
    setMobile(isMobile);
    window.addEventListener("resize", checkIsMobile);
    return () => window.removeEventListener("resize", checkIsMobile);
  }, [isMobile, setMobile]);

  return <ChatMain />;
}
